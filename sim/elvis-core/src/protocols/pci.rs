//! The base-level protocol that communicates directly with networks.

use crate::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Mac, Mtu},
    protocol::{DemuxError, StartError},
    Control, FxDashMap, Network, Protocol, Session, Shutdown,
};
use std::sync::{Arc, OnceLock};
use tokio::sync::Barrier;

pub mod pci_session;
pub(crate) use pci_session::PciSession;

/// A number used to identify protocols which communicate over Ethernet.
///
/// Examples:
/// - IPv4 is identified as `0x0800`.
/// - ARP is identified as `0x0806`.
/// - IPv6 is identified as `0x086DD`.
///
/// See [`EtherTypes`] for an enum containing some of the popular types.
///
/// See <https://en.wikipedia.org/wiki/EtherType> for more information.
pub type EtherType = u16;

/// Some constants you can use to fill in an [`EtherType`].
#[repr(u16)]
pub enum EtherTypes {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
}

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
pub struct Pci {
    /// The protocols that have called .listen on this Pci.
    listen_bindings: FxDashMap<EtherType, Arc<dyn Protocol>>,
    /// Item #0 of the vec represents Pci Slot 0,
    /// Item #1 of the vec represents Pci Slot 1, etc.
    connections: Vec<Connection>,
    protocols: OnceLock<ProtocolMap>,
}

impl Pci {
    /// Creates a new network tap.
    pub fn new(networks: impl IntoIterator<Item = Arc<Network>>) -> Self {
        // Create a `Connection` object for each network
        let networks = networks.into_iter();
        let expected_size = networks.size_hint().1.unwrap_or(0);
        let mut connections = Vec::with_capacity(expected_size);
        for network in networks {
            let conn = Connection {
                mac: network.next_mac(),
                network: network.clone(),
            };
            connections.push(conn);
        }

        Self {
            listen_bindings: FxDashMap::default(),
            connections,
            protocols: OnceLock::new(),
        }
    }

    /// Creates a new session.
    ///
    /// # Arguments
    ///
    /// * `slot`: the PCI slot (interface number)
    /// that this machine will send messages out of.
    /// For example, if you put the number 2,
    /// this will send messages through the second network specified
    /// when you created this PCI with [`Pci::new`].
    ///
    /// * `destination` - the destination MAC address.
    ///
    /// * `protocol` - the [`EtherType`] number of the destination protocol.
    ///
    /// # Panics
    ///
    /// The user should ensure that the slot is less than [`Pci::slot_count`]
    pub fn open(
        &self,
        slot: PciSlot,
        destination: Option<Mac>,
        protocol: EtherType,
    ) -> Arc<PciSession> {
        let connection = self.connections[slot].clone();
        let sesh = PciSession {
            slot,
            connection,
            destination,
            protocol,
        };
        Arc::new(sesh)
    }

    /// When Pci receives a message on the given Pci slot
    /// and intended for the given EtherType
    /// after calling this function,
    /// it will send these messages to the upstream protocol
    /// using `upstream.demux`.
    ///
    /// On success, returns `true`.
    /// If a protocol had already been bound to the given `ethertype`,
    /// this returns `false` and does not perform any binding.
    pub fn listen(&self, upstream: Arc<dyn Protocol>, ethertype: EtherType) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.listen_bindings.entry(ethertype) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(upstream);
                true
            }
        }
    }

    pub fn slot_count(&self) -> usize {
        self.connections.len()
    }

    pub fn mac_addresses(&self) -> impl Iterator<Item = Mac> + '_ {
        self.connections.iter().map(|connection| connection.mac)
    }

    /// Creates a new network tap.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Called by the [`Network`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Protocol::demux`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(self: &Arc<Self>, slot: PciSlot, delivery: Delivery) {
        // get upstream protocol
        let protocol = match self.listen_bindings.get(&delivery.protocol) {
            Some(protocol) => Arc::clone(&protocol),
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {0:?}",
                    delivery.protocol
                );
                return;
            }
        };

        // Create a `control` and put in DemuxInfo
        // so upstream protocols have info about the message they received
        let mut control = Control::new();
        let pci_demux_info = DemuxInfo {
            slot,
            source: delivery.sender,
            destination: delivery.destination,
            mtu: self.connections[slot].network.mtu,
        };
        control.insert(pci_demux_info);

        // make a session so demux can work
        // Currently, this makes a new session each time a message is received.
        // We'll have to think about efficiency on this one...
        let session = PciSession {
            slot,
            connection: self.connections[slot].clone(),
            destination: Some(delivery.sender),
            protocol: delivery.protocol,
        };
        let session = Arc::new(session);

        // send upstream
        let pmap = self
            .protocols
            .get()
            .expect("start should be called before receive");
        let result = protocol.demux(delivery.message, session, control, pmap.clone());
        if let Err(e) = result {
            tracing::error!("Error demuxing upstream from PCI: {}", e);
        }
    }
}

#[async_trait::async_trait]
impl Protocol for Pci {
    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        panic!("Cannot demux on a Pci")
    }

    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let self_arc = protocols
            .protocol::<Pci>()
            .expect("Pci should be in its own ProtocolMap on start");

        let _ = self.protocols.set(protocols);

        // Tell network about self
        for slot in 0..self.connections.len() {
            let connection = &self.connections[slot];
            connection
                .network
                .register_tap(connection.mac, self_arc.clone(), slot);
        }
        initialized.wait().await;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemuxInfo {
    /// The PCI slot that was received on
    pub slot: PciSlot,
    /// The sender's MAC address
    pub source: Mac,
    /// The local MAC address the message was sent to or none to indicate a broadcast message
    pub destination: Option<Mac>,
    pub mtu: Mtu,
}

/// Represents something akin to a NIC.
/// Each PCI slot has a connection.
///
/// * `mac` - the MAC address associated with this connection
/// * `network` - the network this NIC is connected to.
#[derive(Clone)]
struct Connection {
    pub mac: Mac,
    pub network: Arc<Network>,
}
