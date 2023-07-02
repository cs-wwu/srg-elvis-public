use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, Tcp, Udp},
    Control, Protocol, Session, Shutdown, Transport,
};
use std::sync::{Arc, RwLock};
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

#[async_trait::async_trait]
impl Protocol for Capture {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        match self.transport {
            Transport::Tcp => {
                protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols)
                    .unwrap();
            }
            Transport::Udp => {
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols)
                    .unwrap();
            }
        }

        *self.shutdown.write().unwrap() = Some(shutdown);
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
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
