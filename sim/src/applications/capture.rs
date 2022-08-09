use crate::{
    core::{message::Message, Control, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress},
        udp::{LocalPort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug, Clone)]
pub struct Capture {
    message: Option<Message>,
    shutdown: Option<Sender<()>>,
    ip_address: Ipv4Address,
    port: u16,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(ip_address: Ipv4Address, port: u16) -> Self {
        Self {
            message: None,
            shutdown: None,
            ip_address,
            port,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(ip_address: Ipv4Address, port: u16) -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(ip_address, port))
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
        LocalAddress::set(&mut participants, self.ip_address);
        LocalPort::set(&mut participants, self.port);
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
