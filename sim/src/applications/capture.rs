use crate::{
    core::{message::Message, Control, ControlFlow, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{cell::RefCell, error::Error, rc::Rc};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    message: Option<Message>,
    did_set_up: bool,
}

impl Capture {
    /// Creates a new capture.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared() -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new())
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.clone()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::from_string("Capture");

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if !self.did_set_up {
            let mut participants = Control::new();
            LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
            RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
            LocalPort::set(&mut participants, 0xbeefu16);
            RemotePort::set(&mut participants, 0xdeadu16);
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
