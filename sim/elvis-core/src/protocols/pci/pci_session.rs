use super::Pci;
use crate::{
    control::{Key, Primitive},
    gcd::{Delivery, GcdHandle},
    internet::NetworkHandle,
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Mac, Mtu, Network},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError},
    Id, Session,
};
use std::sync::{Arc, RwLock};

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    mac: Mac,
    mtu: Mtu,
    slot: PciSlot,
    pub network: NetworkHandle,
    gcd: RwLock<Option<GcdHandle>>,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(network: NetworkHandle, mac: Mac, mtu: Mtu, slot: u32) -> Self {
        Self {
            mac,
            mtu,
            slot,
            network,
            gcd: Default::default(),
        }
    }

    /// Called by the owning [`Pci`] protocol at the beginning of the simulation
    /// to start the contained tap running
    pub(super) fn start(&self, gcd: GcdHandle) {
        *self.gcd.write().unwrap() = Some(gcd);
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(
        self: &Arc<Self>,
        delivery: Delivery,
        protocols: ProtocolMap,
    ) -> Result<(), ReceiveError> {
        let mut context = Context::new(protocols);
        Pci::set_pci_slot(self.slot, &mut context.control);
        Network::set_sender(delivery.sender, &mut context.control);
        let protocol = match context.protocol(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                eprintln!(
                    "Could not find a protocol for the protocol ID {}",
                    delivery.protocol
                );
                Err(ReceiveError::Protocol(delivery.protocol))?
            }
        };
        protocol.demux(delivery.message, self.clone(), context)?;
        Ok(())
    }
}

impl Session for PciSession {
    fn send(&self, message: Message, context: Context) -> Result<(), SendError> {
        let protocol = match Network::get_protocol(&context.control) {
            Ok(protocol) => protocol,
            Err(_) => {
                eprintln!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = Network::get_destination(&context.control).ok();

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

        self.gcd
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .delivery(delivery);
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
