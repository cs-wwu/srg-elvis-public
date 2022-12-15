//! The [`Session`] trait and supporting types.

use crate::control::{Key, Primitive};

use super::{protocol::Context, Message};
use std::sync::Arc;

/// A shared handle to a [`Session`]
pub type SharedSession = Arc<dyn Session + Send + Sync + 'static>;

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
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), ()>;

    /// Takes an incoming message and decides which protocol to send it to for
    /// further processing.
    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), ()>;

    /// Gets a piece of information from some session in the protocol stack.
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, ()>;
}
