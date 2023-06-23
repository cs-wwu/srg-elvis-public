//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//!
//! [`Arp`] is also used to do subnetting. See [`Arp::set_subnet`] and [`subnetting`] for more info.
//! Currently, ELVIS/Arp do not support *any* of the reserved Ipv4 addresses.
pub mod arp_parsing;
pub mod internals;
pub mod subnetting;

use self::internals::*;

use std::sync::Arc;

use tokio::sync::Barrier;

use crate::machine::ProtocolMap;
use crate::protocol::{DemuxError, StartError, NotifyType};
use crate::protocols::ipv4::*;
use crate::session::SendError;
use crate::{machine::PciSlot, network::Mac, Control, Message, Protocol, Session, Shutdown};

use self::arp_parsing::{ArpPacket, Operation};
use self::subnetting::SubnetInfo;

use super::pci::DemuxInfo;

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
pub struct Arp {
    /// The ARP innards.
    /// I wanted to be able to implement 2 versions of ARP: a debug version and a normal version.
    /// Rust doesn't support inheritance, so I did this instead
    /// Is this a terrible idea? I hope not
    inner: Box<dyn ArpInner>,
}

impl Arp {
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
    /// (This is for the Ipv4 protocol, because it wants to send messages to the router's MAC address
    /// in this case.)
    ///
    /// If no default gateway is specified in the subnet info, this returns `Err`.
    pub async fn resolve(
        &self,
        endpoints: AddressPair,
        tap_slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Result<Mac, NoResponseError> {
        self.inner.resolve(endpoints, tap_slot, protocols).await
    }

    /// Adds the given IP address to this ARP session's list of local IPs.
    /// Arp will ignore arp requests and replies that it recieves
    /// if they do not match one of its local IPs.
    /// (Will also be set by the [`Arp::resolve`] method.)
    pub fn listen(&self, local_ip: Ipv4Address) {
        self.inner.listen(local_ip);
    }

    /// Sets the subnet of a local IP address,
    /// for use with the [`Arp::resolve`] method.
    pub fn set_subnet(&self, local_ip: Ipv4Address, subnet: SubnetInfo) {
        self.inner.set_subnet(local_ip, subnet);
    }

    /// Sets the subnet of a local IP address.
    /// This is used to configure subnets before running a simulation. If you want to
    /// configure subnets while running the simulation, use [`Arp::set_subnet`].
    pub fn preconfig_subnet(self, local_ip: Ipv4Address, subnet: SubnetInfo) -> Self {
        self.set_subnet(local_ip, subnet);
        self
    }

    /// Creates an [`Arp`] from the given [`ArpInner`].
    /// This allows you replace Arp with your own drop-in implementation.
    /// See [`internals`] for more information.
    pub fn from_inner(inner: impl ArpInner) -> Arp {
        Arp::from(inner)
    }

    /// Creates a basic [`Arp`] object.
    pub fn basic() -> Arp {
        Self::from(BasicArpInner::new())
    }

    /// Creates a Debug Arp object.
    ///
    /// When [`DebugArpInner::resolve`] is called, `resolve_hook` will also be called.
    ///
    /// When [`DebugArpInner::demux`] is called, `demux_hook` will also be called.
    pub fn debug<R, D>(resolve_hook: R, demux_hook: D) -> Self
    where
        R: Fn(AddressPair, PciSlot) + Send + Sync + 'static,
        D: Fn(Message) + Send + Sync + 'static,
    {
        Self::from(DebugArpInner::new(resolve_hook, demux_hook))
    }
}

#[async_trait::async_trait]
impl Protocol for Arp {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        self.inner.start(shutdown, initialized, protocols)
    }

    /// This should be called when an ARP request or reply is received.
    /// Processes the given Message (which should be an IPv4 over ethernet ARP packet).
    /// Adds the sender's IP and MAC to the ARP table.
    /// If this is an Arp request, send back an Arp reply.
    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        self.inner.demux(message, caller, control, protocols)
    }

    fn notify(&self, _notification: NotifyType, _caller: Arc<dyn Session>, _control: Control) {}
}

impl<I: ArpInner> From<I> for Arp {
    fn from(value: I) -> Arp {
        Arp {
            inner: Box::new(value) as Box<dyn ArpInner>,
        }
    }
}

impl Default for Arp {
    /// Alias for [`Self::basic`].
    fn default() -> Self {
        Self::basic()
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
