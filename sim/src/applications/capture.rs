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
    message: Arc<Mutex<Option<Message>>>,
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    ip_address: Ipv4Address,
    port: u16,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(ip_address: Ipv4Address, port: u16) -> Self {
        Self {
            message: Default::default(),
            shutdown: Default::default(),
            ip_address,
            port,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(ip_address: Ipv4Address, port: u16) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(ip_address, port))
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.lock().unwrap().clone()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::from_string("Capture");

    fn start(
        self: Arc<Self>,
        context: ProtocolContext,
        shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        *self.shutdown.lock().unwrap() = Some(shutdown);
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, self.ip_address);
        LocalPort::set(&mut participants, self.port);
        context
            .protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, context)?;
        Ok(())
    }

    fn recv(
        self: Arc<Self>,
        message: Message,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        *self.message.lock().unwrap() = Some(message);
        if let Some(shutdown) = self.shutdown.lock().unwrap().take() { tokio::spawn(async move {
                shutdown.send(()).await.unwrap();
            }); }
        Ok(())
    }
}
