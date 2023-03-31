use super::Pci;
use crate::{
    control::{Key, Primitive},
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Tap},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError},
    Id, Network, Session,
};
use std::sync::{Arc, RwLock};

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    tap: Tap,
    index: PciSlot,
    protocols: RwLock<Option<ProtocolMap>>,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(tap: Tap, index: u32) -> Arc<Self> {
        let network = tap.network.clone();
        let mac = tap.mac;
        let this = Self {
            tap,
            index,
            protocols: Default::default(),
        };
        let this = Arc::new(this);
        network.register_tap(mac, this.clone());
        this
    }

    /// Called by the owning [`Pci`] protocol at the beginning of the simulation
    /// to start the contained tap running
    pub(super) fn start(self: Arc<Self>, protocols: ProtocolMap) {
        *self.protocols.write().unwrap() = Some(protocols);
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(self: Arc<Self>, delivery: Delivery) -> Result<(), ReceiveError> {
        let mut context = Context::new(self.protocols.read().unwrap().as_ref().unwrap().clone());
        Pci::set_pci_slot(self.index, &mut context.control);
        Network::set_sender(delivery.sender, &mut context.control);
        let protocol = match context.protocol(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {}",
                    delivery.protocol
                );
                Err(ReceiveError::Protocol(delivery.protocol))?
            }
        };
        protocol.demux(delivery.message, self, context)?;
        Ok(())
    }
}

impl Session for PciSession {
    #[tracing::instrument(name = "PciSession::send", skip_all)]
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), SendError> {
        let protocol = match Network::get_protocol(&context.control) {
            Ok(protocol) => protocol,
            Err(_) => {
                tracing::error!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = Network::get_destination(&context.control).ok();

        if message.len() > self.tap.mtu as usize {
            tracing::error!("Attempted to send a message larger than the network can handle");
            Err(SendError::Mtu(self.tap.mtu))?
        }

        let funnel = self.tap.delivery_sender.clone();
        let delivery = Delivery {
            message,
            sender: self.tap.mac,
            destination,
            protocol,
        };

        tokio::spawn(async move {
            match funnel.send(delivery).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to send on direct network: {}", e);
                }
            }
        });

        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Pci::MTU_QUERY_KEY => Ok(self.tap.mtu.into()),
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
