use super::Pci;
use crate::{
    control::{Key, Primitive},
    gcd::{self, get_protocol, Delivery},
    internet::NetworkHandle,
    machine::PciSlot,
    message::Message,
    network::{Mac, Mtu, Network},
    protocol::DemuxError,
    session::{QueryError, SendError},
    Control, Id, Session,
};
use std::sync::Arc;

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    mac: Mac,
    mtu: Mtu,
    slot: PciSlot,
    pub network: NetworkHandle,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(network: NetworkHandle, mac: Mac, mtu: Mtu, slot: u32) -> Self {
        Self {
            mac,
            mtu,
            slot,
            network,
        }
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(self: &Arc<Self>, delivery: Delivery) -> Result<(), ReceiveError> {
        let mut control = Control::new();
        Pci::set_pci_slot(self.slot, &mut control);
        Network::set_sender(delivery.sender, &mut control);
        let protocol = match get_protocol(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                eprintln!(
                    "Could not find a protocol for the protocol ID {}",
                    delivery.protocol
                );
                Err(ReceiveError::Protocol(delivery.protocol))?
            }
        };
        protocol.demux(delivery.message, self.clone(), control)?;
        Ok(())
    }
}

impl Session for PciSession {
    fn send(&self, message: Message, control: Control) -> Result<(), SendError> {
        let protocol = match Network::get_protocol(&control) {
            Ok(protocol) => protocol,
            Err(_) => {
                eprintln!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = Network::get_destination(&control).ok();

        if message.len() > self.mtu as usize {
            eprintln!("Attempted to send a message larger than the network can handle");
            Err(SendError::Mtu(self.mtu))?
        }

        let delivery = Delivery {
            message,
            sender: self.mac,
            destination,
            protocol,
            network: self.network,
        };

        gcd::delivery(delivery);
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Pci::MTU_QUERY_KEY => Ok(self.mtu.into()),
            _ => Err(QueryError::MissingKey),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Could not find a protocol for the given id: {0}")]
    Protocol(Id),
    #[error("{0}")]
    Demux(#[from] DemuxError),
}
