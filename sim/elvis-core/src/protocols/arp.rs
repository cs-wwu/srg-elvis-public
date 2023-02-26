//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//! In ELVIS, the Ipv4Sessions connect with ARP.
//! Arp will fetch MAC addresses when query'd.

pub mod arp_parsing;
pub mod arp_session;

use std::{collections::HashMap, sync::Arc};

use crate::{
    control::{Control, Key, Primitive},
    machine::PciSlot,
    network::Mac,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::Pci,
    session::SharedSession,
    Id, Message, Protocol, ProtocolMap,
};

use self::arp_session::ArpSession;

use super::{ipv4::Ipv4Address, Ipv4};

use dashmap::DashMap;
use tokio::sync::{mpsc::Sender, watch, Barrier};

pub struct Arp {
    /// The ARP table, or cache. Maps Ipv4 addresses to MAC addresses.
    pub arp_table: DashMap<Ipv4Address, Mac>,
    /// When an ARP packet is received, a () will be sent through this
    arp_received_sender: watch::Sender<()>,
    /// When an ARP packet is recieved, this channel will receive ()
    arp_received_receiver: watch::Receiver<()>,
}

impl Arp {
    /// A unique identifier for the protocol. (0x0806 is the EtherType value of ARP)
    pub const ID: Id = Id::new(0x0806);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        let (arp_received_sender, arp_received_receiver) = watch::channel(());
        Self {
            arp_table: Default::default(),
            arp_received_sender,
            arp_received_receiver,
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Gets a destination MAC address from this machine's ARP Protocol, if it exists.
    ///
    /// # Arguments:
    ///
    /// * `local_ip`: The IP address of this machine.
    /// * `remote_ip`: The IP address to get a MAC address for.
    /// * `slot`: The tap slot that will be used to communicate with the remote IP address.
    /// * `protocols`: a ProtocolMap for this machine.
    ///
    /// # Returns:
    ///
    /// * `Some(mac)` if it was able to resolve a MAC address.
    /// * `None` otherwise.
    pub fn query_mac_address(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Option<Mac> {
        let arp = protocols.protocol(Arp::ID)?;

        let mut participants = Control::new();
        Pci::set_pci_slot(slot, &mut participants);
        Ipv4::set_local_address(local_ip, &mut participants);
        Ipv4::set_remote_address(remote_ip, &mut participants);

        let arp_session = arp
            .open(Self::ID, participants, protocols)
            .expect("unable to create ARP session");

        let dest_mac = arp_session
            .query((Arp::ID, remote_ip.to_u32() as u64))
            .expect("unable to obtain MAC from ARP session")
            .to_u64()
            .expect("unable to unwrap u64");

        // do arp_session.close() when close is invented
        Some(dest_mac)
    }
}

impl Protocol for Arp {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn start(
        self: Arc<Self>,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        todo!()
    }

    /// The participants set must contain a pci slot, a local IPv4 address, and a remote IPv4 address.
    fn open(
        self: Arc<Self>,
        _upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let slot = Pci::get_pci_slot(&participants).expect("participants must have PCI slot");
        let local_ip =
            Ipv4::get_local_address(&participants).expect("participants must have local IP");
        let remote_ip =
            Ipv4::get_remote_address(&participants).expect("participants must have remote IP");
        let downstream = protocols
            .protocol(Pci::ID)
            .expect("no such protocol")
            .open(Self::ID, participants, protocols)?;
        Ok(Arc::new(ArpSession::new(
            slot, local_ip, remote_ip, self, downstream,
        )))
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        Ok(())
    }

    /// In general, this will be called by the Pci layer when an ARP packet is recieved
    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        todo!()
    }

    /// Arp cannot be queried or it will panic.
    /// If you want a MAC address, you should query an ArpSession.
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
