use super::Pci;
use crate::{
    control::{Key, Primitive},
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Mac},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError},
    Id, Network, Session,
};
use std::sync::{Arc, RwLock};

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    mac: Mac,
    index: PciSlot,
    network: Arc<Network>,
    protocols: RwLock<Option<ProtocolMap>>,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(network: Arc<Network>, index: u32) -> Arc<Self> {
        let mac = network.next_mac();
        let this = Self {
            mac,
            index,
            network: network.clone(),
            protocols: Default::default(),
        };
        let this = Arc::new(this);
        network.register_tap(mac, this.clone());
        this
    }

    /// Called by the owning [`Pci`] protocol at the beginning of the simulation
    /// to start the contained tap running
    pub(super) fn start(&self, protocols: ProtocolMap) {
        *self.protocols.write().unwrap() = Some(protocols);
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(self: &Arc<Self>, delivery: Delivery) -> Result<(), ReceiveError> {
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
        protocol.demux(delivery.message, self.clone(), context)?;
        Ok(())
    }
}

impl Session for PciSession {
    #[tracing::instrument(name = "PciSession::send", skip_all)]
    fn send(&self, message: Message, context: Context) -> Result<(), SendError> {
        let protocol = match Network::get_protocol(&context.control) {
            Ok(protocol) => protocol,
            Err(_) => {
                tracing::error!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = Network::get_destination(&context.control).ok();

        if message.len() > self.network.mtu as usize {
            tracing::error!("Attempted to send a message larger than the network can handle");
            Err(SendError::Mtu(self.network.mtu))?
        }

        let delivery = Delivery {
            message,
            sender: self.mac,
            destination,
            protocol,
        };

        let network = self.network.clone();
        tokio::spawn(async move {
            network.send(delivery).await;
        });
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Pci::MTU_QUERY_KEY => Ok(self.network.mtu.into()),
            Pci::MAC_QUERY_KEY => Ok(self.mac.into()),
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
