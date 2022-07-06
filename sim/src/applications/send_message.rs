use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{
    core::{Control, ControlFlow, ControlKey, Message, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{Application, Ipv4Address, Udp, UserProcess},
};

pub struct SendMessage {
    text: &'static str,
    did_set_up: bool,
}

impl SendMessage {
    pub fn new(text: &'static str) -> Self {
        Self { text, did_set_up: false }
    }

    pub fn new_shared(text: &'static str) -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(text))
    }
}

impl Application for SendMessage {
    const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 1);

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if self.did_set_up {
            return Ok(ControlFlow::Continue)
        }
        self.did_set_up = true;

        let protocol = context.protocol(Udp::ID)?;
        let participants = Control::new()
            // Todo: This should be some other IP address
            .with(ControlKey::LocalAddress, Ipv4Address::LOCALHOST.to_u32())
            .with(ControlKey::RemoteAddress, Ipv4Address::LOCALHOST.to_u32())
            .with(ControlKey::LocalPort, 0xdeadu16)
            .with(ControlKey::RemotePort, 0xbeefu16);
        let session = protocol
            .borrow_mut()
            .open_active(Self::ID, participants, context)?;
        session
            .borrow_mut()
            .send(session.clone(), Message::new(self.text), context)?;
        Ok(ControlFlow::Continue)
    }

    fn recv(
        &mut self,
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
