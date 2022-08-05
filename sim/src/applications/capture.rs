use tokio::sync::mpsc::Sender;

use crate::{
    core::{message::Message, Control, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug, Default, Clone)]
pub struct Capture {
    message: Option<Message>,
    shutdown: Option<Sender<()>>,
}

impl Capture {
    /// Creates a new capture.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared() -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new())
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.clone()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::from_string("Capture");

    fn start(
        &mut self,
        mut context: ProtocolContext,
        shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        self.shutdown = Some(shutdown);
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0xbeefu16);
        RemotePort::set(&mut participants, 0xdeadu16);
        context
            .protocol(Udp::ID)
            .expect("No such protocol")
            .lock()
            .unwrap()
            .listen(Self::ID, participants, &mut context)?;
        Ok(())
    }

    fn recv(
        &mut self,
        message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.message = Some(message);
        let shutdown = self.shutdown.take().unwrap();
        tokio::spawn(async move {
            shutdown.send(()).await.unwrap();
        });
        Ok(())
    }
}
