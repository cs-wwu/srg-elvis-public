//! The [`Session`] trait and supporting types.

use super::Message;
use crate::{machine::ProtocolMap, network::Mtu};
use thiserror::Error as ThisError;

/// Holds the state for a particular connection.
///
/// A [`Protocol`](super::Protocol) creates a session to respond to a particular
/// connection. For example, an IP protocol might create a session to handle
/// messages with a particular pair of local and remote addresses. A session is
/// in charge of appending headers to outgoing messages, deciding which protocol
/// to use for demuxing incoming messages, and keeping track of state such as
/// TCP windows.
pub trait Session: Send + Sync + 'static {
    /// Takes the message, appends headers, and forwards it to the next session
    /// in the chain for further processing.
    fn send(&self, message: Message, protocols: ProtocolMap) -> Result<(), SendError>;
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    #[error("Failed to construct a valid header for the payload")]
    Header,
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("The message length exceeds the network's MTU: {0}")]
    Mtu(Mtu),
    #[error("Unspecified error")]
    Other,
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    #[error("No session held a datum matching the given key")]
    MissingKey,
}
