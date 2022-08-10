use super::{Message, ProtocolContext};
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
    /// Takes the message, appends headers, and forwards it to the next session
    /// in the chain for further processing.
    fn send(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>>;

    /// Takes an incoming message and decides which protocol to send it to for
    /// further processing.
    fn receive(&mut self, message: Message, context: ProtocolContext)
        -> Result<(), Box<dyn Error>>;

    fn start(&mut self, context: ProtocolContext) -> Result<(), Box<dyn Error>>;
}
