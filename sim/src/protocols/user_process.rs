use crate::core::{
    Control, ControlFlow, Message, Protocol, ProtocolContext, ProtocolId, RcSession,
};
use std::{cell::RefCell, error::Error, rc::Rc};
use thiserror::Error as ThisError;

pub trait Application {
    const ID: ProtocolId;

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>>;

    fn recv(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct UserProcess<T: Application> {
    application: T,
}

impl<T: Application> UserProcess<T> {
    pub fn new(application: T) -> Self {
        Self { application }
    }

    pub fn new_shared(application: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(application)))
    }

    pub fn application(&self) -> &T {
        &self.application
    }
}

impl<T: Application> Protocol for UserProcess<T> {
    fn id(&self) -> ProtocolId {
        T::ID
    }

    fn open_active(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<RcSession, Box<dyn Error>> {
        Err(UserError::OpenActive)?
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(UserError::Listen)?
    }

    fn demux(
        &mut self,
        message: Message,
        _downstream: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.application.recv(message, context)
    }

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        self.application.awake(context)
    }
}

#[derive(Debug, ThisError)]
pub enum UserError {
    #[error("Cannot open_active on a user program")]
    OpenActive,
    #[error("Cannot listen on a user program")]
    Listen,
}
