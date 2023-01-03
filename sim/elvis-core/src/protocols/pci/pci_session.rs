use tokio::sync::Barrier;

use super::Pci;
use crate::{
    control::{Key, Primitive},
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Tap, TapEnvironment},
    protocol::Context,
    session::{QueryError, ReceiveError, SendError},
    Network, Session,
};
use std::sync::Arc;

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    tap: Tap,
    index: PciSlot,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(tap: Tap, index: u32) -> Self {
        Self { tap, index }
    }

    pub(super) fn start(self: Arc<Self>, protocols: ProtocolMap, barrier: Arc<Barrier>) {
        let environment = TapEnvironment::new(protocols, self.clone());
        self.tap.start(environment, barrier);
    }

    pub(crate) fn receive_pci(
        self: Arc<Self>,
        delivery: Delivery,
        mut context: Context,
    ) -> Result<(), ReceiveError> {
        Pci::set_pci_slot(self.index, &mut context.control);
        Network::set_sender(delivery.sender, &mut context.control);
        let protocol = match context.protocol(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {}",
                    delivery.protocol
                );
                Err(ReceiveError::Other)?
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
        self.tap.send(message, destination, protocol)?;
        Ok(())
    }

    #[tracing::instrument(name = "PciSession::receive", skip_all)]
    fn receive(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), ReceiveError> {
        panic!("Use receive_pci insteaed")
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.tap.query(key)
    }
}
