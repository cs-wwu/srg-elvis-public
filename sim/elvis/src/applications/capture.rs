use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress},
        udp::LocalPort,
        user_process::{Application, UserProcess},
        Udp,
    },
    Control,
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
    /// The message that was received, if any
    message: Arc<Mutex<Option<Message>>>,
    /// The channel we send on to shut down the simulation
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
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
        context: Context,
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

    fn recv(self: Arc<Self>, message: Message, _context: Context) -> Result<(), Box<dyn Error>> {
        *self.message.lock().unwrap() = Some(message);
        if let Some(shutdown) = self.shutdown.lock().unwrap().take() {
            tokio::spawn(async move {
                shutdown.send(()).await.unwrap();
            });
        }
        Ok(())
    }
}
