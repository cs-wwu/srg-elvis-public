//! Utilities for running user-level programs in the context of a
//! protocol-oriented simulation.
//!
//!
//!
//!
//! Due to the nature of the [`async_trait::async_trait`] macro,
//! this looks like a mess when viewed with `cargo doc`.
//! When you create your own application, you can do it like so:
//!
//! ```
//! use elvis_core::*;
//! use elvis_core::machine::*;
//! use elvis_core::session::Session;
//! use elvis_core::protocols::user_process::*;
//! use tokio::sync::Barrier;
//! use std::sync::Arc;
//!
//! struct MyApp {}
//!
//! #[async_trait::async_trait]
//! impl Application for MyApp {
//!     async fn start(
//!         &self,
//!         shutdown: Shutdown,
//!         initialize: Arc<Barrier>,
//!         protocols: ProtocolMap,
//!     ) -> Result<(), ApplicationError> {
//!         Ok(())
//!     }
//!
//!     fn receive(
//!         &self,
//!         message: Message,
//!         caller: Arc<dyn Session>,
//!         control: Control,
//!         protocols: ProtocolMap,
//!     ) -> Result<(), ApplicationError> {
//!         Ok(())
//!     }
//! }
//! ```

use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    session::SendError,
    Control, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;
use tracing::error;

/// A program being run in a [`UserProcess`].
///
/// An application is similar to a stripped-down
/// [`Session`](crate::Session). It runs when messages come in over the
/// network or when the containing machine awakens the
/// application to give it time to run.
#[async_trait::async_trait]
pub trait Application {
    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    async fn start(
        &self,
        shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError>;

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn receive(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError>;
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationError {
    #[error("A send call failed")]
    Send(#[from] SendError),
    #[error("Missing protocol {0:?}")]
    MissingProtocol(TypeId),
    #[error("Unspecified error")]
    Other,
}

/// A user-level process that sits at the top of the networking stack.
///
/// In Elvis, user-level processes are protocols like anything else. Unlike most
/// protocols, they do not have sessions associated with them. Instead, when
/// messages are demuxed to a user process, they are sent to the [`Application`]
/// assigned to the generic type parameter `A`. Also unlike other protocols,
/// user processes should not have higher-level protocols attempting to open
/// connections on or listen through them.
#[derive(Debug, Clone)]
pub struct UserProcess<A: Application + Send + Sync + 'static> {
    application: A,
}

impl<A: Application + Send + Sync + 'static> UserProcess<A> {
    /// Creates a new user process to run the given application.
    pub fn new(application: A) -> Self {
        Self { application }
    }

    /// Creates a new user process running the given application behind a shared
    /// handle.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Gets the application the user process is running.
    pub fn application(&self) -> &A {
        &self.application
    }
}

#[async_trait::async_trait]
impl<A: Application + Send + Sync + 'static> Protocol for UserProcess<A> {
    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        self.application
            .receive(message, caller, control, protocols)?;
        Ok(())
    }

    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        self.application
            .start(shutdown, initialized, protocols)
            .await?;
        Ok(())
    }
}
