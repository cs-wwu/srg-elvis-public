use crate::{
    core::{message::Message, Control, ControlFlow, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{self, Ipv4Address},
        udp::{self, Udp},
        user_process::{Application, UserProcess},
    },
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
        if !self.did_set_up {
            let participants = Control::new()
                .with(ipv4::LOCAL_ADDRESS_KEY, Ipv4Address::LOCALHOST.to_u32())
                .with(ipv4::REMOTE_ADDRESS_KEY, Ipv4Address::LOCALHOST.to_u32())
                .with(udp::LOCAL_PORT_KEY, 0xbeefu16)
                .with(udp::REMOTE_PORT_KEY, 0xdeadu16);
            context
                .protocol(Udp::ID)
                .expect("No such protocol")
                .borrow_mut()
                .listen(Self::ID, participants, context)?;
        }
        self.did_set_up = true;

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
