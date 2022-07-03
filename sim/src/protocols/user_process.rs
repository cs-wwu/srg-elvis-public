use crate::core::{
    ArcSession, Control, ControlFlow, Message, NetworkLayer, Protocol, ProtocolContext, ProtocolId,
};
use std::error::Error;
use thiserror::Error as ThisError;

pub trait Application {
    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>>;
}

pub struct UserProcess {
    application: Box<dyn Application>,
}

impl UserProcess {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    pub fn new(application: Box<dyn Application>) -> Self {
        Self { application }
    }
}

impl Protocol for UserProcess {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(UserError::OpenActive)?
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(UserError::Listen)?
    }

    fn demux(
        &mut self,
        _message: Message,
        _downstream: ArcSession,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(UserError::Demux)?
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        self.application.awake(context)
    }
}

#[derive(Debug, ThisError)]
pub enum UserError {
    #[error("Cannot open_active on a user program")]
    OpenActive,
    #[error("Cannot listen on a user program")]
    Listen,
    #[error("Cannot demux on a user program")]
    Demux,
}
