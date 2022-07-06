use crate::{
    core::{Control, ControlFlow, ControlKey, Message, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{Application, Ipv4Address, Udp, UserProcess},
};
use std::{cell::RefCell, error::Error, rc::Rc};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    message: Option<Message>,
    did_set_up: bool,
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

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if self.did_set_up {
            return Ok(ControlFlow::Continue)
        }
        self.did_set_up = true;

        let participants = Control::new()
            .with(ControlKey::LocalAddress, Ipv4Address::LOCALHOST.to_u32())
            .with(ControlKey::RemoteAddress, Ipv4Address::LOCALHOST.to_u32())
            .with(ControlKey::LocalPort, 0xbeefu16)
            .with(ControlKey::RemotePort, 0xdeadu16);
        context
            .protocol(Udp::ID)?
            .borrow_mut()
            .listen(Self::ID, participants, context)?;
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
