//! Utilities for running user-level programs in the context of a
//! protocol-oriented simulation.

use tokio::sync::mpsc::Sender;

use crate::core::{
    message::Message, Control, Protocol, ProtocolContext, ProtocolId, SharedSession,
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

/// A program being run in a [`UserProcess`].
///
/// An application is similar to a stripped-down
/// [`Session`](crate::core::Session). It runs when messages come in over the
/// network or when the containing machine awakens the
/// application to give it time to run.
pub trait Application {
    /// A unique identifier for the application.
    const ID: ProtocolId;

    /// Gives the application time to run. Unlike [`recv`](Self::recv), `awake`
    /// is not called in response to specific events.
    fn start(
        &mut self,
        context: ProtocolContext,
        shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>>;

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn recv(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;
}

/// A user-level process that sits at the top of the networking stack.
///
/// In Elvis, user-level processes are protocols like anything else. Unlike most
/// protocols, they do not have sessions associated with them. Instead, when
/// messages are demuxed to a user process, they are sent to the [`Application`]
/// assigned to the generic type parameter `A`. Also unlike other protocols,
/// user processes should not have higher-level protocols attempting to open
/// connections on or listen through them.
pub struct UserProcess<A: Application> {
    application: A,
}

impl<A: Application> UserProcess<A> {
    /// Creates a new user process to run the given application.
    pub fn new(application: A) -> Self {
        Self { application }
    }

    /// Creates a new user process running the given application behind a shared
    /// handle.
    pub fn new_shared(application: A) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(application)))
    }

    /// Gets the application the user process is running.
    pub fn application(&self) -> &A {
        &self.application
    }
}

impl<A: Application> Protocol for UserProcess<A> {
    fn id(&self) -> ProtocolId {
        A::ID
    }

    fn open(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        panic!("Cannot active open on a user process")
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Cannot listen on a user process")
    }

    fn demux(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.application.recv(message, context)
    }

    fn start(
        &mut self,
        context: ProtocolContext,
        shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        self.application.start(context, shutdown)
    }
}
