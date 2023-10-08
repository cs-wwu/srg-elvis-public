use crate::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Mac, Mtu},
    protocol::DemuxError,
    session::SendError,
    Session,
};
use std::any::TypeId;

use super::Connection;

/// The session type associated with [`Pci`](super::Pci).
/// Contains information about a connection between a PCI slot and the network.
pub struct PciSession {
    /// The local Pci slot associated with this connection.
    pub(super) slot: PciSlot,
    /// The connection to the network.
    pub(super) connection: Connection,
    /// The destination MAC address.
    pub(super) destination: Option<Mac>,
    /// The EtherType of the destination protocol.
    pub(super) protocol: super::EtherType,
}
impl PciSession {
    pub fn mac(&self) -> Mac {
        self.connection.mac
    }

    pub fn slot(&self) -> PciSlot {
        self.slot
    }

    pub fn mtu(&self) -> Mtu {
        self.connection.network.mtu
    }

    /// Similar to [`PciSession::send`],
    /// but does not require a ProtocolMap.
    pub fn send_pci(&self, message: Message) -> Result<(), SendError> {
        if message.len() > self.connection.network.mtu as usize {
            return Err(SendError::Mtu(self.connection.network.mtu));
        }

        let delivery = Delivery {
            message,
            sender: self.connection.mac,
            destination: self.destination,
            protocol: self.protocol,
        };

        let network = self.connection.network.clone();
        tokio::spawn(async move {
            network.send(delivery).await;
        });

        Ok(())
    }
}

impl Session for PciSession {
    fn send(&self, message: Message, _protocols: ProtocolMap) -> Result<(), SendError> {
        self.send_pci(message)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Could not find a protocol for the given id: {0:?}")]
    Protocol(TypeId),
    #[error("{0}")]
    Demux(#[from] DemuxError),
}
