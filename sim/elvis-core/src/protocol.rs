//! The [`Protocol`] trait and supporting types.

use super::message::Message;
use crate::{session::SendError, Control, Machine, Session, Shutdown};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use futures::Future;
use tokio::sync::Barrier;

// TODO(hardint): Should add a str argument to the Other variant of errors so
// that the reason for an error shows up in traces and such.

/// A member of a networking protocol stack.
///
/// A protocol is responsible for creating new [`Session`]s and
/// demultiplexing requests to the correct session.
pub trait Protocol: Send + Sync + 'static {
    fn id(&self) -> TypeId {
        self.type_id()
    }

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
    /// 
    /// # Using async
    /// 
    /// The return type of this method is quite ugly. We suggest writing
    /// your start method as an async fn.
    /// ([This is totally legal!](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html#can-i-mix-async-fn-and-impl-trait))
    /// 
    /// ```
    /// # use elvis_core::protocol::*;
    /// # use elvis_core::shutdown::*;
    /// # use elvis_core::*;
    /// # use elvis_core::session::*;
    /// # use tokio::sync::Barrier;
    /// # use std::sync::Arc;
    /// pub struct MyProtocol {}
    /// impl Protocol for MyProtocol {
    ///     async fn start(
    ///         &self,
    ///         _shutdown: Shutdown,
    ///         initialized: Arc<Barrier>,
    ///         _machine: Arc<Machine>,
    ///     ) -> Result<(), StartError> {
    ///         initialized.wait().await;
    ///         Ok(())
    ///     }
    /// #
    /// #   fn demux(
    /// #       &self,
    /// #       _: Message,
    /// #       _: Arc<dyn Session>,
    /// #       _: Control,
    /// #       _: Arc<Machine>,
    /// #   ) -> Result<(), DemuxError> { Ok(()) }
    /// }
    /// ```
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> impl Future<Output = Result<(), StartError>> + Send
    where
        Self: Sized;

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
    ///   asked to receive the message by calling `listen` at an earlier time.
    ///   (Most protocols, such as `Ipv4` and `Udp`, have a `listen` or
    ///   `open_and_listen` function.)
    ///   If so, a new session should be created.
    /// - Call `receive` on the selected session.
    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError>;

    /// Allows for notifying a protocol about an occurrence,
    /// Eg. a new connection being established
    fn notify(&self, _notification: NotifyType, _caller: Arc<dyn Session>, _control: Control) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyType {
    NewConnection,
    NewMessage,
}

// TODO(hardint): Get rid of these error types and replace them with inline logging

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum NotifyError {
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("Unspecified query error")]
    Other,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DemuxError {
    #[error("Failed to find a session to demux to")]
    MissingSession,
    #[error("The session was closed")]
    ClosedSession,
    #[error("Data expected through the context was missing")]
    MissingContext,
    #[error("Could not find the given protocol: {0:?}")]
    MissingProtocol(TypeId),
    #[error("Failed to parse a header during demux")]
    Header,
    #[error("Unspecified demux error")]
    Other,
}

impl From<SendError> for DemuxError {
    fn from(value: SendError) -> DemuxError {
        match value {
            SendError::Header => DemuxError::Header,
            SendError::MissingContext => DemuxError::MissingContext,
            SendError::Mtu(_) => DemuxError::Other,
            SendError::Other => DemuxError::Other,
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    #[error("Could not find the given protocol: {0:?}")]
    MissingProtocol(TypeId),
    #[error("Unspecified error")]
    Other,
}
