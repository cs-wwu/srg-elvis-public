//! The [`Protocol`] trait and supporting types.

use super::{message::Message, session::SharedSession, Control};
use crate::{
    control::{Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    protocols::user_process::ApplicationError,
    session::SendError,
    Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

mod context;
pub use context::Context;

// TODO(hardint): Should add a str argument to the Other variant of errors so
// that the reason for an error shows up in traces and such.

/// A shared handle to a [`Protocol`].
pub type SharedProtocol = Arc<dyn Protocol + Send + Sync>;

/// A member of a networking protocol stack.
///
/// A protocol is responsible for creating new [`Session`](super::Session)s and
/// demultiplexing requests to the correct session.
pub trait Protocol {
    /// Returns a unique identifier for the protocol.
    fn id(self: Arc<Self>) -> Id;

    /// Starts the protocol running. This gives protocols an opportunity to open
    /// sessions, spawn tasks, and perform other setup as needed.
    ///
    /// All implementors should wait on the barrier after completing synchronous
    /// operations such as opening sessions or spawning tasks and, critically,
    /// before sending anything on the network. This allows applications that
    /// may wish to send messages to delay until the moment that other machines
    /// are ready to receive the message. Implementors may also store the
    /// `shutdown` channel and send on it at a later time to cleanly shut down
    /// the simulation.
    fn start(
        self: Arc<Self>,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError>;

    /// Actively open a new network connection.
    ///
    /// Called by the `upstream` protocol to create a new
    /// [`Session`](super::Session) for a connection. Each protocol should, in
    /// turn, `open` a session with some downstream protocol to establish a
    /// chain of sessions with which to send and receive messages for the
    /// requesting user program.
    ///
    /// The `participants` set contains key-value pairs that identify aspects of
    /// a connection to facilitate [`demux`](Protocol::demux)ing. It should
    /// contain all attributes needed to uniquely identify the connection. For
    /// example, an IP protocol might require the attributes `{local_address,
    /// remote_address}`. A UDP or TCP protocol might require the attributes
    /// `{local_address, local_port, remote_address, remote_port}`.
    fn open(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError>;

    /// Listen for new connections.
    ///
    /// Requests that messages for which there is no existing session be sent to
    /// the `upstream` protocol. Only messages that match the `participants` set
    /// will be forwarded. See [`demux`](Protocol::demux) for more details.
    ///
    /// The participants set should contain all attributes needed to identify
    /// the listening program. For example, an IP protocol might use the set of
    /// attributes `{local_address}`. Since we are listening for connections
    /// from any remote address, when the IP protocol sees a message it does not
    /// have a session for, it will check whether the local address given in the
    /// header is one it is listening for. If so, it will create the session
    /// identified by `{local_address, remote_address}` and continue
    /// demultiplexing the message. Similarly, a UDP or TCP protocol would want
    /// its participant set to include {local_address, local_port}.
    fn listen(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError>;

    /// Identifies the session that a message belongs to and forwards the
    /// message to it.
    ///
    /// When demultiplexing a message, a protocol will typically carry out
    /// several tasks:
    ///
    /// - Remove and parse the message header.
    /// - Apply information about the message header to the context. This should
    ///   include any information that the target session or other protocols may
    ///   need to know about. For example, an IP protocol should add the source
    ///   and destination addresses to the context so that UDP and TCP may use
    ///   them for verifying checksums.
    /// - Select a session to respond to the message. This is done by looking at
    ///   information extracted from the header. If there is no matching
    ///   session, the protocol should check to see whether any protocol has
    ///   asked to receive the message by calling [`listen`](Protocol::listen)
    ///   at an earlier time. If so, a new session should be created.
    /// - Call `receive` on the selected session.
    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError>;

    /// Gets a piece of information from the protocol
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError>;
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    #[error("The provided key cannot be queried on this protocol")]
    NonexistentKey,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DemuxError {
    #[error("Failed to find a session to demux to")]
    MissingSession,
    #[error("The session was closed")]
    ClosedSession,
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("Could not find the given protocol: {0}")]
    MissingProtocol(Id),
    #[error("Failed to parse a header during demux")]
    Header,
    #[error("Receive failed during the execution of an Application")]
    Application(#[from] ApplicationError),
    #[error("Failed to open a session during demux")]
    Open(#[from] OpenError),
    #[error("Failed to send a message during demux")]
    Send(#[from] SendError),
    #[error("Unspecified demux error")]
    Other,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ListenError {
    #[error("The listen binding already exists")]
    Existing,
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("Unspecified error")]
    Other,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    #[error("Protocol failed to start because an application failed to start")]
    Application(#[from] ApplicationError),
    #[error("Unspecified error")]
    Other,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    #[error("The session already exists")]
    Existing,
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("Send failed while opening a session: {0}")]
    Send(#[from] SendError),
    #[error("Unspecified error")]
    Other,
}
