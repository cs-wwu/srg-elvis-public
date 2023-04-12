//! Utilities for running user-level programs in the context of a
//! protocol-oriented simulation.

use crate::{
    control::{Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    message::Message,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    session::{SendError, SharedSession},
    Control, Protocol, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;
use tracing::error;

/// A program being run in a [`UserProcess`].
///
/// An application is similar to a stripped-down
/// [`Session`](crate::Session). It runs when messages come in over the
/// network or when the containing machine awakens the
/// application to give it time to run.
pub trait Application {
    /// A unique identifier for the application.
    const ID: Id;

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
    fn receive(&self, message: Message, context: Context) -> Result<(), ApplicationError>;
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationError {
    #[error("A listen call failed")]
    Listen(#[from] ListenError),
    #[error("An open call failed")]
    Open(#[from] OpenError),
    #[error("A send call failed")]
    Send(#[from] SendError),
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
    fn id(&self) -> Id {
        A::ID
    }

    fn open(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        panic!("Cannot active open on a user process")
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        panic!("Cannot listen on a user process")
    }

    fn demux(
        self: Arc<Self>,
        message: Message,
        _caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        self.application.receive(message, context)?;
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

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
