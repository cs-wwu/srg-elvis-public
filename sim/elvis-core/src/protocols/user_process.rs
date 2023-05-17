//! Utilities for running user-level programs in the context of a
//! protocol-oriented simulation.

use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    session::{SendError, SharedSession},
    Control, Protocol, Shutdown,
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
pub trait Application {
    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    fn start(
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

impl<A: Application + Send + Sync + 'static> Protocol for UserProcess<A> {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn demux(
        &self,
        message: Message,
        _caller: SharedSession,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        self.application.receive(message, control, protocols)?;
        Ok(())
    }

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        self.application.start(shutdown, initialized, protocols)?;
        Ok(())
    }
}
