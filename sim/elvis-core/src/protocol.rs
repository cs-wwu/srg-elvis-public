//! The [`Protocol`] trait and supporting types.
//!
//!
//! # Async trait
//!
//! Due to the nature of the [`async_trait::async_trait`] macro,
//! this looks like a mess when viewed with `cargo doc`.
//! When you create your own application, you can do it like so:
//!
//! ```
//! use elvis_core::*;
//! use elvis_core::machine::*;
//! use elvis_core::session::Session;
//! use elvis_core::protocol::*;
//! use tokio::sync::Barrier;
//! use std::sync::Arc;
//! use std::any::*;
//!
//! struct MyApp {}
//!
//! #[async_trait::async_trait]
//! impl Protocol for MyApp {
//!     fn id(&self) -> TypeId {
//!         self.type_id()
//!     }
//!     async fn start(
//!         &self,
//!         shutdown: Shutdown,
//!         initialize: Arc<Barrier>,
//!         protocols: ProtocolMap,
//!     ) -> Result<(), StartError> {
//!         Ok(())
//!     }
//!
//!     fn demux(
//!         &self,
//!         message: Message,
//!         caller: Arc<dyn Session>,
//!         control: Control,
//!         protocols: ProtocolMap,
//!     ) -> Result<(), DemuxError> {
//!         Ok(())
//!     }
//! }
//! ```

use super::message::Message;
use crate::{
    machine::ProtocolMap, protocols::user_process::ApplicationError, Control, Session, Shutdown,
};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tokio::sync::Barrier;

// TODO(hardint): Should add a str argument to the Other variant of errors so
// that the reason for an error shows up in traces and such.

/// A member of a networking protocol stack.
///
/// A protocol is responsible for creating new [`Session`](super::Session)s and
/// demultiplexing requests to the correct session.
#[async_trait::async_trait]
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
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError>;

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
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError>;
}

// TODO(hardint): Get rid of these error types and replace them with inline logging

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
    #[error("Receive failed during the execution of an Application")]
    Application(#[from] ApplicationError),
    #[error("Unspecified demux error")]
    Other,
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    #[error("Protocol failed to start because an application failed to start")]
    Application(#[from] ApplicationError),
    #[error("Unspecified error")]
    Other,
}
