use super::{Message, ProtocolContext, ProtocolId};
use std::error::Error;

/// Holds the state for a particular connection.
///
/// A [`Protocol`](super::Protocol) creates a session to respond to a particular
/// connection. For example, an IP protocol might create a session to handle
/// messages with a particular pair of local and remote addresses. A session is
/// in charge of appending headers to outgoing messages, deciding which protocol
/// to use for demuxing incoming messages, and keeping track of state such as
/// TCP windows.
pub trait Session {
    /// Returns the ID of the protocol that creates and manages this session.
    fn protocol(&self) -> ProtocolId;

    /// Takes the message, appends headers, and forwards it to the next session
    /// in the chain for further processing.
    fn send(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    /// Takes an incoming message and decides which protocol to send it to for
    /// further processing.
    fn receive(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    /// Called to allow a session to carry out some work outside the context of
    /// responding to a message.
    ///
    /// As an example, TCP may decide to retransmit packets or poll empty window
    /// sizes even when no new messages are being sent or received. This
    /// lifecycle method is a session's opportunity to carry out such tasks.
    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>>;
}

/// Expresses what to do after a protocol is called on to run.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControlFlow {
    /// Keep running the simulation
    #[default]
    Continue,
    /// Stop running the simulation
    EndSimulation,
}
