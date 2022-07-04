use crate::{
    core::{ControlFlow, Message, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{Application, UserProcess},
};
use std::{cell::RefCell, error::Error, rc::Rc};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    message: Option<Message>,
}

impl Capture {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new())
    }

    pub fn message(&self) -> Option<Message> {
        self.message.clone()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(if self.message.is_some() {
            ControlFlow::EndSimulation
        } else {
            ControlFlow::Continue
        })
    }

    fn recv(
        &mut self,
        message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.message = Some(message);
        Ok(())
    }
}
