use super::Pci;
use crate::{
    control::{Key, Primitive},
    machine::{ProtocolMap, TapSlot},
    message::Message,
    network::{SharedTap, TapEnvironment},
    protocol::{Context, ProtocolId},
    session::{QueryError, ReceiveError, SendError},
    Session,
};
use std::sync::Arc;

/// The session type for a [`Tap`](super::Tap).
pub(crate) struct PciSession {
    tap: SharedTap,
    index: TapSlot,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(tap: SharedTap, index: u32) -> Self {
        Self { tap, index }
    }

    pub(super) fn start(self: Arc<Self>, protocols: ProtocolMap) {
        let environment = TapEnvironment::new(protocols, self.clone());
        self.tap.clone().start(environment);
    }
}

impl Session for PciSession {
    #[tracing::instrument(name = "PciSession::send", skip_all)]
    fn send(self: Arc<Self>, mut message: Message, context: Context) -> Result<(), SendError> {
        let first_responder = match Pci::get_first_responder(&context.info) {
            Ok(first_responder) => first_responder,
            Err(_) => {
                tracing::error!("First responder missing from context");
                Err(SendError::MissingContext)?
            }
        };
        message.prepend(first_responder.into_inner().to_be_bytes().to_vec());
        self.tap.clone().send(message, context.info)?;
        Ok(())
    }

    #[tracing::instrument(name = "PciSession::receive", skip_all)]
    fn receive(
        self: Arc<Self>,
        mut message: Message,
        mut context: Context,
    ) -> Result<(), ReceiveError> {
        let first_responder = match take_header(&message) {
            Some(protocol) => protocol,
            None => {
                tracing::error!("Expected eight bytes for the tap header");
                Err(ReceiveError::Other)?
            }
        };
        message.slice(8..);
        Pci::set_first_responder(first_responder, &mut context.info);
        Pci::set_tap_slot(self.index, &mut context.info);
        let protocol = match context.protocol(first_responder) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {}",
                    first_responder
                );
                Err(ReceiveError::Other)?
            }
        };
        protocol.demux(message, self, context)?;
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        // TODO(hardint): Add support for querying the MTU
        // TODO(hardint): Add support for querying the machine ID
        Err(QueryError::MissingKey)
    }
}

/// Parses the Tap header from the message, which is just the ID of the protocol
/// that should receive this message.
fn take_header(message: &Message) -> Option<ProtocolId> {
    let mut iter = message.iter();
    Some(
        u64::from_be_bytes([
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
        ])
        .into(),
    )
}
