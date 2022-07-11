use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{
    core::{message::Message, Control, ControlFlow, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{self, Ipv4Address},
        udp::{self, Udp},
        user_process::{Application, UserProcess},
    },
};

pub struct SendMessage {
    text: &'static str,
    did_set_up: bool,
}

impl SendMessage {
    pub fn new(text: &'static str) -> Self {
        Self {
            text,
            did_set_up: false,
        }
    }

    pub fn new_shared(text: &'static str) -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(text))
    }
}

impl Application for SendMessage {
    const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 1);

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if self.did_set_up {
            return Ok(ControlFlow::Continue);
        }
        self.did_set_up = true;

        let participants = Control::new()
            // Todo: This should be some other IP address
            .with(ipv4::LOCAL_ADDRESS_KEY, Ipv4Address::LOCALHOST.to_u32())
            .with(ipv4::REMOTE_ADDRESS_KEY, Ipv4Address::LOCALHOST.to_u32())
            .with(udp::LOCAL_PORT_KEY, 0xdeadu16)
            .with(udp::REMOTE_PORT_KEY, 0xbeefu16);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let mut session = protocol
            .borrow_mut()
            .open_active(Self::ID, participants, context)?;
        session.send(Message::new(self.text), context)?;
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
