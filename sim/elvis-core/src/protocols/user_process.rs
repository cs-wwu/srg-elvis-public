//! Utilities for running user-level programs in the context of a
//! protocol-oriented simulation.

use crate::{
    control::{Key, Primitive},
    message::Message,
    protocol::{Context, ProtocolId},
    session::SharedSession,
    Control, Protocol,
};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc::Sender, Barrier};

/// A program being run in a [`UserProcess`].
///
/// An application is similar to a stripped-down
/// [`Session`](crate::Session). It runs when messages come in over the
/// network or when the containing machine awakens the
/// application to give it time to run.
pub trait Application {
    /// A unique identifier for the application.
    const ID: ProtocolId;

    /// Gives the application time to run. Unlike [`recv`](Self::recv), `awake`
    /// is not called in response to specific events.
    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialize: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>>;

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn recv(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>>;
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
    application: Arc<A>,
}

impl<A: Application + Send + Sync + 'static> UserProcess<A> {
    /// Creates a new user process to run the given application.
    pub fn new(application: A) -> Self {
        Self {
            application: Arc::new(application),
        }
    }

    /// Creates a new user process running the given application behind a shared
    /// handle.
    pub fn new_shared(application: A) -> Arc<Self> {
        Arc::new(Self::new(application))
    }

    /// Gets the application the user process is running.
    pub fn application(&self) -> Arc<A> {
        self.application.clone()
    }
}

impl<A: Application + Send + Sync + 'static> Protocol for UserProcess<A> {
    fn id(self: Arc<Self>) -> ProtocolId {
        A::ID
    }

    fn open(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        panic!("Cannot active open on a user process")
    }

    fn listen(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Cannot listen on a user process")
    }

    fn demux(
        self: Arc<Self>,
        message: Message,
        _caller: SharedSession,
        context: Context,
    ) -> Result<(), Box<dyn Error>> {
        let application = self.application.clone();
        match application.recv(message, context) {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e),
        }
        Ok(())
    }

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>> {
        let application = self.application.clone();
        match application.start(context, shutdown, initialized) {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e),
        }
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, Box<dyn Error>> {
        panic!("Cannot query a user process")
    }
}
