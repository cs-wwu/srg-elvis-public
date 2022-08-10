use super::{control::make_key, message::Message, Control, ProtocolContext, SharedSession};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;

/// A unique identifier for a [`Protocol`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolId(u64);

impl ProtocolId {
    /// Creates a new protocol ID with the given number.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Creates a pseudorandom ID by hashing the string identifier.
    pub const fn from_string(string: &'static str) -> Self {
        Self(make_key(string))
    }

    /// Gets the underlying ID number.
    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl From<u64> for ProtocolId {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

impl From<ProtocolId> for u64 {
    fn from(id: ProtocolId) -> Self {
        id.0
    }
}

/// A shared handle to a [`Protocol`].
pub type SharedProtocol = Arc<Mutex<dyn Protocol + Send + Sync>>;

/// A member of a networking protocol stack.
///
/// A protocol is responsible for creating new [`Session`](super::Session)s and
/// demultiplexing requests to the correct session.
pub trait Protocol {
    // TODO(hardint): We need methods that allow other protocols to query info about a
    // protocol and its sessions. For example, a TCP or an IP protocol will want
    // a method to learn about a Tap's MTU.

    /// Returns a unique identifier for the protocol.
    fn id(&self) -> ProtocolId;

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
        &self,
        upstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>>;

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
        &self,
        upstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

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
    /// - Call [`receive`](super::Session::receive) on the selected session.
    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>>;

    fn start(&self, context: ProtocolContext, shutdown: Sender<()>) -> Result<(), Box<dyn Error>>;
}
