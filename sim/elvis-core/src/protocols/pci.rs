//! The base-level protocol that communicates directly with networks.

use crate::{
    machine::{Machine, PciSlot},
    message::Message,
    network::{Mac, Mtu},
    protocol::{DemuxError, StartError},
    Control, Network, Protocol, Session, Shutdown, internet::DoneSender,
};
use std::sync::Arc;

pub mod pci_session;
pub(crate) use pci_session::PciSession;

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
pub struct Pci {
    sessions: Vec<Arc<PciSession>>,
}

impl Pci {
    /// Creates a new network tap.
    pub fn new(networks: impl IntoIterator<Item = Arc<Network>>) -> Self {
        Self {
            sessions: networks
                .into_iter()
                .enumerate()
                .map(|(i, network)| PciSession::new(network, i as u32))
                .collect(),
        }
    }

    /// Gets the PCI session for the given slot.
    ///
    /// # Panics
    /// The user should ensure that the slot is less than [`Pci::slot_count`]
    pub fn open(&self, slot: PciSlot) -> Arc<PciSession> {
        self.sessions.get(slot as usize).unwrap().clone()
    }

    pub fn slot_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn mac_addresses(&self) -> impl Iterator<Item = Mac> + '_ {
        self.sessions.iter().map(|session| session.mac())
    }

    /// Creates a new network tap.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

#[async_trait::async_trait]
impl Protocol for Pci {
    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        panic!("Cannot demux on a Pci")
    }

    async fn boot(
        &self,
        shutdown: Shutdown,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        for session in self.sessions.iter() {
            session.start(machine.clone());
        }
        Ok(())
    }

    async fn start(
        &self,
        _shutdown: Shutdown,
        init_done: DoneSender,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        init_done.send(());
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
