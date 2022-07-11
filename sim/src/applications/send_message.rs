use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{
    core::{message::Message, Control, ControlFlow, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{set_local_address, set_remote_address, Ipv4Address},
        udp::{set_local_port, set_remote_port, Udp},
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

        let mut participants = Control::new();
        // Todo: This should be some other IP address
        set_local_address(&mut participants, Ipv4Address::LOCALHOST);
        set_remote_address(&mut participants, Ipv4Address::LOCALHOST);
        set_local_port(&mut participants, 0xdeadu16);
        set_remote_port(&mut participants, 0xbeefu16);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let mut session = protocol
            .borrow_mut()
            .open(Self::ID, participants, context)?;
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
