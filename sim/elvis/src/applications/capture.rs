use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
    },
    Control, Id, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

use super::Transport;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct Capture {
    /// The message that was received, if any
    message: RwLock<Option<Message>>,
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
    /// The number of messages it will receive before stopping
    message_count: u32,
    /// The number of messages currently recieved
    cur_count: RwLock<u32>,
    /// The transport protocol to use
    transport: Transport,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(ip_address: Ipv4Address, port: u16, message_count: u32) -> Self {
        Self {
            message: Default::default(),
            shutdown: Default::default(),
            ip_address,
            port,
            message_count,
            cur_count: RwLock::new(0),
            transport: Transport::Udp,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.read().unwrap().clone()
    }

    /// Set the transport protocol to use
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for Capture {
    const ID: Id = Id::from_string("Capture");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        let mut participants = Control::new();
        participants.local.address = Some(self.ip_address);
        participants.local.port = Some(self.port);
        protocols
            .protocol(self.transport.id())
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)?;
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(&self, message: Message, _context: Context) -> Result<(), ApplicationError> {
        *self.message.write().unwrap() = Some(message);
        *self.cur_count.write().unwrap() += 1;
        if *self.cur_count.read().unwrap() >= self.message_count {
            if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                shutdown.shut_down();
            }
        }
        Ok(())
    }
}
