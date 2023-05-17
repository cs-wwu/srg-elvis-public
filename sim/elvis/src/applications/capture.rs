use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        user_process::{Application, ApplicationError, UserProcess},
        Endpoint, Tcp, Udp,
    },
    Control, Shutdown, Transport,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct Capture {
    /// The message that was received, if any
    message: RwLock<Option<Message>>,
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    endpoint: Endpoint,
    /// The number of messages it will receive before stopping
    message_count: u32,
    /// The number of messages currently recieved
    cur_count: RwLock<u32>,
    /// The transport protocol to use
    transport: Transport,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(endpoint: Endpoint, message_count: u32) -> Self {
        Self {
            message: Default::default(),
            shutdown: Default::default(),
            endpoint,
            message_count,
            cur_count: RwLock::new(0),
            transport: Transport::Udp,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
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
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        match self.transport {
            Transport::Tcp => {
                protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(TypeId::of::<UserProcess<Self>>(), self.endpoint, protocols)
                    .unwrap();
            }
            Transport::Udp => {
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(TypeId::of::<UserProcess<Self>>(), self.endpoint, protocols)
                    .unwrap();
            }
        }

        *self.shutdown.write().unwrap() = Some(shutdown);
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
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
