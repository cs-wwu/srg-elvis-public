//! Tools for replacing Arp's behavior with your own.
//!
//! If you don't plan to change Arp's behavior, you can use the existing [`Arp::basic`] and
//! [`Arp::debug`] functions to create simple versions of [`Arp`]
//! instead of worrying about the stuff in this module.
use std::any::TypeId;
use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use dashmap::mapref::entry::Entry;
use tokio::sync::{watch, Barrier};

use super::*;
use crate::machine::ProtocolMap;
use crate::protocol::{DemuxError, StartError};
use crate::protocols::arp::subnetting::*;
use crate::protocols::{ipv4::*, pci::pci_session::PciSession, Pci};
use crate::session::SendError;
use crate::{machine::PciSlot, network::Mac, Control, FxDashMap, Message, Session, Shutdown};

/// A trait for structs that can be used as the internals of Arp.
#[async_trait]
pub trait ArpInner: Send + Sync + 'static {
    /// Called by [`Arp::start`].
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError>;

    /// Called by [`Arp::resolve`].
    async fn resolve(
        &self,
        endpoints: AddressPair,
        tap_slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Result<Mac, NoResponseError>;

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError>;

    fn listen(&self, local_ip: Ipv4Address);

    fn set_subnet(&self, local_ip: Ipv4Address, subnet: SubnetInfo);
}

/// The basic ArpInner. Functions like [`Arp`] says it should.
#[derive(Default)]
pub struct BasicArpInner {
    /// This machine's local IPs
    local_ips: FxDashMap<Ipv4Address, Option<SubnetInfo>>,
    /// The ARP table that maps IP addresses to MACs
    arp_table: ArpTable,
}

impl BasicArpInner {
    /// The duration to wait before sending another ARP request
    pub const RESEND_DELAY: Duration = Duration::from_millis(200);
    /// The number of times we should try sending ARP requests before giving up
    pub const RESEND_TRIES: u32 = 10;

    /// Creates a new `BasicArpInner` for use in [`Arp::from_inner`].
    /// Instead of using this function, you should probably use [`Arp::basic`].
    pub fn new() -> Self {
        Default::default()
    }
}

fn send_arp_request(pci_session: Arc<PciSession>, addrs: AddressPair) -> Result<(), SendError> {
    let local_mac = pci_session.mac();
    let packet = ArpPacket::new_request(local_mac, addrs.local, addrs.remote);
    pci_session.send_pci(packet.build().into(), None, TypeId::of::<Arp>())
}

#[async_trait]
impl ArpInner for BasicArpInner {
    async fn resolve(
        &self,
        mut endpoints: AddressPair,
        tap_slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Result<Mac, NoResponseError> {
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
        let pci_session = protocols
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

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move { initialized.wait().await });
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // try to convert message to arp packet
        let packet = match ArpPacket::from_bytes(message.iter()) {
            Ok(message) => message,
            Err(e) => {
                tracing::error!("Failed to parse ARP packet: {}", e);
                return Ok(());
            }
        };

        // discard packet if we are not recipient
        if !self.local_ips.contains_key(&packet.target_ip) {
            return Ok(());
        }

        // put packet's ip and mac in the table
        self.arp_table.set_mac(packet.sender_ip, packet.sender_mac);

        // if it was a request, send a reply
        let request = packet;
        if request.oper == Operation::Request {
            let sesh_info = control
                .get::<DemuxInfo>()
                .expect("Context should contain PCI demux info");
            let new_sesh = protocols
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

    fn set_subnet(&self, local_ip: Ipv4Address, subnet: SubnetInfo) {
        self.local_ips.insert(local_ip, Some(subnet));
    }

    fn listen(&self, local_ip: Ipv4Address) {
        // This is important to make sure we don't destroy the subnetting info when adding our local ip
        if let Entry::Vacant(entry) = self.local_ips.entry(local_ip) {
            entry.insert(None);
        }
    }
}

/// A wrapper around [`BasicArpInner`]. You can set custom functions to be called
/// when a message is received. For example, you could set a function that prints out the messages received.
/// Useful for debugging.
pub struct DebugArpInner {
    inner: BasicArpInner,
    resolve_hook: Box<dyn Fn(AddressPair, PciSlot) + Send + Sync + 'static>,
    demux_hook: Box<dyn Fn(Message) + Send + Sync + 'static>,
}

impl DebugArpInner {
    /// Creates a DebugArpInner.
    ///
    /// When [`DebugArpInner::resolve`] is called, `resolve_hook` will also be called.
    ///
    /// When [`DebugArpInner::demux`] is called, `demux_hook` will also be called.
    pub fn new<R, D>(resolve_hook: R, demux_hook: D) -> Self
    where
        R: Fn(AddressPair, PciSlot) + Send + Sync + 'static,
        D: Fn(Message) + Send + Sync + 'static,
    {
        Self {
            inner: BasicArpInner::new(),
            resolve_hook: Box::new(resolve_hook),
            demux_hook: Box::new(demux_hook),
        }
    }
}

#[async_trait]
impl ArpInner for DebugArpInner {
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        self.inner.start(shutdown, initialized, protocols)
    }

    async fn resolve(
        &self,
        endpoints: AddressPair,
        tap_slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Result<Mac, NoResponseError> {
        (self.resolve_hook)(endpoints, tap_slot);
        self.inner.resolve(endpoints, tap_slot, protocols).await
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        (self.demux_hook)(message.clone());
        self.inner.demux(message, caller, control, protocols)
    }

    fn listen(&self, local_ip: Ipv4Address) {
        self.inner.listen(local_ip)
    }

    fn set_subnet(&self, local_ip: Ipv4Address, subnet: SubnetInfo) {
        self.inner.set_subnet(local_ip, subnet)
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
