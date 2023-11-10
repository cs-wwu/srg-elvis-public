//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//!
//! [`Arp`] is also used to do subnetting. See [`Arp::set_subnet`] and [`subnetting`] for more info.
//! Currently, ELVIS/Arp do not support *any* of the reserved Ipv4 addresses.
pub mod arp_parsing;
pub mod subnetting;

use std::any::TypeId;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::sync::Barrier;

use crate::protocol::{DemuxError, StartError};
use crate::protocols::ipv4::*;
use crate::session::SendError;
use crate::FxDashMap;
use crate::Machine;
use crate::{machine::PciSlot, network::Mac, Control, Message, Protocol, Session, Shutdown};

use self::arp_parsing::{ArpPacket, Operation};
use self::subnetting::{Ipv4Net, SubnetInfo};

use super::pci::{DemuxInfo, PciSession};
use super::Pci;

/// Arp stands for Address Resolution Protocol. Its job is to figure out another (Ipv4-using) machine's MAC
/// address, and send messages to that MAC, instead of broadcasting them to the whole network.
///
/// In ELVIS, Arp sits (optionally) between the Ipv4 and Pci protocols.
/// Using Arp is rather simple. Just add it to your machine.
///
/// ```ignore
/// new_machine!([
///     Udp::new(),
///     Ipv4::new(std::iter::empty().collect()),
///     Arp::basic(),
///     Pci::new([]),
/// ])
/// ```
///
/// Ipv4 will then use ARP to figure out which destination MAC address
/// a message should be sent to (instead of broadcasting it).
///
/// The machine you are sending messages to MUST also have an Arp protocol, and a local IP address
/// (set by [`Ipv4::listen`] or [`Arp::listen`]).

/// The basic ArpInner. Functions like [`Arp`] says it should.
#[derive(Default)]
pub struct Arp {
    /// This machine's local IPs
    local_ips: FxDashMap<Ipv4Address, Option<SubnetInfo>>,
    /// The ARP table that maps IP addresses to MACs
    arp_table: ArpTable,
    /// These functions get called when resolve() and demux() are called,
    /// respectively
    resolve_hook: Option<Box<dyn Fn(AddressPair, PciSlot) + Send + Sync + 'static>>,
    demux_hook: Option<Box<dyn Fn(Message) + Send + Sync + 'static>>,
}

fn send_arp_request(pci_session: Arc<PciSession>, addrs: AddressPair) -> Result<(), SendError> {
    let local_mac = pci_session.mac();
    let packet = ArpPacket::new_request(local_mac, addrs.local, addrs.remote);
    pci_session.send_pci(packet.build().into(), None, TypeId::of::<Arp>())
}

#[async_trait::async_trait]
impl Protocol for Arp {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        if let Some(demux_hook) = &self.demux_hook {
            demux_hook(message.clone());
        }

        // try to convert message to arp packet
        let packet = match ArpPacket::from_bytes(message.iter()) {
            Ok(message) => message,
            Err(e) => {
                tracing::error!("Failed to parse ARP packet: {}", e);
                return Ok(());
            }
        };

        // put packet's ip and mac in the table
        self.arp_table.set_mac(packet.sender_ip, packet.sender_mac);

        // if it was a request, send a reply
        let request = packet;
        if request.oper == Operation::Request && self.local_ips.contains_key(&request.target_ip) {
            let sesh_info = control
                .get::<DemuxInfo>()
                .expect("Context should contain PCI demux info");
            let new_sesh = machine
                .protocol::<Pci>()
                .expect("Pci should be in protocols")
                .open(sesh_info.slot);
            let reply = ArpPacket::new_reply(
                new_sesh.mac(),
                request.target_ip, // the reply's sender IP (us) is the request's target IP
                request.sender_mac,
                request.sender_ip,
            );
            let result = new_sesh.send_pci(
                reply.build().into(),
                Some(request.sender_mac),
                TypeId::of::<Arp>(),
            );

            if let Err(e) = result {
                tracing::error!("failed to send ARP reply: {:?}", e);
            }
        }

        Ok(())
    }
}

impl Arp {
    /// The duration to wait before sending another ARP request
    pub const RESEND_DELAY: Duration = Duration::from_millis(200);
    /// The number of times we should try sending ARP requests before giving up
    pub const RESEND_TRIES: u32 = 10;

    /// Attempts to resolve the MAC address of the given IP address.
    ///
    /// # Arguments
    ///
    /// * `endpoints` - the local (this machine) and remote (target) IP address.
    /// This IP address will be added to the list of local IPs to listen to.
    ///
    /// * `tap_slot` - the tap slot the target machine can be reached through
    ///
    /// # Return value
    ///
    /// Returns `Ok(mac)` if the MAC address of the remote machine could be resolved.
    /// Otherwise returns `Err(NoResponseErr)`.
    ///
    /// # Subnetting information
    ///
    /// By default, Arp treats all machines as if they are on the same network.
    /// If you would like to use subnetting, then you can use the [`Arp::set_subnet`] function
    /// to set a subnet for one of your local IPs.
    ///
    /// If the local IP address is on the same network as the remote address (according to the subnet info),
    /// then `resolve` will resolve the MAC address of that machine.
    ///
    /// If the local IP address is not on the same network as the remote address (according to the subnet info),
    /// then this will resolve the MAC address of the router.
    ///
    /// If no default gateway is specified in the subnet info, this returns `Err`.
    pub async fn resolve(
        &self,
        mut endpoints: AddressPair,
        tap_slot: PciSlot,
        machine: Arc<Machine>,
    ) -> Result<Mac, NoResponseError> {
        if let Some(resolve_hook) = &self.resolve_hook {
            resolve_hook(endpoints, tap_slot);
        }

        self.listen(endpoints.local);
        // SUBNETTING:
        // If the given endpoints.remote is on the same network as this one,
        // then we want to resolve its MAC address.
        // If the given endpoints.remote is NOT on the same network,
        // then we want to resolve the ROUTER's MAC address, because
        // we would want to send our messages to that.
        let subnet = {
            match self.local_ips.get(&endpoints.local) {
                Some(inner) => *inner,
                None => None,
            }
        };

        if let Some(subnet) = subnet {
            let mask = subnet.mask;
            if Ipv4Net::new(endpoints.local, mask).id() != Ipv4Net::new(endpoints.remote, mask).id()
            {
                endpoints.remote = subnet.default_gateway;
            }
        };

        let dest_ip = endpoints.remote;
        // if the mac can be resolved right away, return that
        if let Some(status) = self.arp_table.get_clone(dest_ip) {
            return status;
        }

        // otherwise, send out arp requests and wait
        let pci_session = machine
            .protocol::<Pci>()
            .expect("Pci should be in protocols")
            .open(tap_slot);

        for _ in 0..Self::RESEND_TRIES {
            send_arp_request(pci_session.clone(), endpoints)?;
            // wait before sending another request
            let result =
                tokio::time::timeout(Self::RESEND_DELAY, self.arp_table.get_mac(dest_ip)).await;
            if let Ok(status) = result {
                return status;
            }
        }
        self.arp_table.fail_mac(dest_ip);

        Err(NoResponseError)
    }

    /// Sets the subnet of a local IP address.
    /// This is used to configure subnets before running a simulation. If you want to
    /// configure subnets while running the simulation, use [`Arp::set_subnet`].
    pub fn preconfig_subnet(self, local_ip: Ipv4Address, subnet: SubnetInfo) -> Self {
        self.set_subnet(local_ip, subnet);
        self
    }

    /// Sets the subnet of a local IP address,
    /// for use with the [`Arp::resolve`] method.
    pub fn set_subnet(&self, local_ip: Ipv4Address, subnet: SubnetInfo) {
        self.local_ips.insert(local_ip, Some(subnet));
    }

    /// Adds the given IP address to this ARP session's list of local IPs.
    /// (Will also be set by the [`Arp::resolve`] method.)
    pub fn listen(&self, local_ip: Ipv4Address) {
        use dashmap::mapref::entry::Entry;
        // This is important to make sure we don't destroy the subnetting info when adding our local ip
        if let Entry::Vacant(entry) = self.local_ips.entry(local_ip) {
            entry.insert(None);
        }
    }

    /// Creates a new [`Arp`] object.
    pub fn new() -> Arp {
        Arp::default()
    }

    /// Registers a callback.
    /// Whenever [`Arp::resolve`] is called,
    /// `func` will also be called.
    pub fn resolve_hook<R>(mut self, func: R) -> Self
    where
        R: Fn(AddressPair, PciSlot) + Send + Sync + 'static,
    {
        self.resolve_hook = Some(Box::new(func));
        self
    }

    /// Registers a callback.
    /// Whenever [`Arp::demux`] is called,
    /// `func` will also be called.
    pub fn demux_hook<D>(mut self, func: D) -> Self
    where
        D: Fn(Message) + Send + Sync + 'static,
    {
        self.demux_hook = Some(Box::new(func));
        self
    }
}

/// A struct representing an arp table.
/// Like a DashMap, but you can wait for a MAC to be set.
pub struct ArpTable {
    table: FxDashMap<Ipv4Address, MacStatus>,
    /// () is sent through this when the table is updated
    update: watch::Sender<()>,
}

type MacStatus = Result<Mac, NoResponseError>;

impl ArpTable {
    pub fn set_mac(&self, ip: Ipv4Address, mac: Mac) {
        self.table.insert(ip, Ok(mac));
        self.update.send_replace(());
    }

    pub fn fail_mac(&self, ip: Ipv4Address) {
        self.table.insert(ip, Err(NoResponseError));
        self.update.send_replace(());
    }

    pub fn remove_mac(&self, ip: Ipv4Address) {
        self.table.remove(&ip);
    }

    /// waits for a mac status to be set, then returns it
    pub async fn get_mac(&self, ip: Ipv4Address) -> MacStatus {
        let mut recv = self.update.subscribe();
        loop {
            if let Some(value) = self.get_clone(ip) {
                return value;
            }

            // if the mac wasn't there, wait for it
            recv.changed().await.expect("sender should not be dropped");
        }
    }

    /// tries to get the current mac status from the arp table.
    /// returns none if a mac status has not been set.
    pub fn get_clone(&self, ip: Ipv4Address) -> Option<MacStatus> {
        self.table.get(&ip).map(|entry| *entry)
    }
}

impl Default for ArpTable {
    fn default() -> Self {
        Self {
            table: FxDashMap::default(),
            update: watch::channel(()).0,
        }
    }
}
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq, Hash)]
#[error("didn't get response for mac address")]
pub struct NoResponseError;

impl From<SendError> for NoResponseError {
    fn from(_: SendError) -> Self {
        Self
    }
}
