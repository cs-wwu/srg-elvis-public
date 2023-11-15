use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Tcp, Udp},
    shutdown::ExitStatus,
    Control, Protocol, Session, Shutdown, Transport,
};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::Barrier;

/// An application that receives a message and increments a shared counter
/// if the shared counter counts to its capacity, the simulation exits
/// can be used to debug message ordering by having each capture return
/// a different exit status
#[derive(Debug)]
pub struct MultiCapture {
    /// The message that was received, if any
    message: RwLock<Option<Message>>,
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    endpoint: Endpoint,
    /// The number of messages currently recieved
    cur_count: RwLock<u32>,
    /// The transport protocol to use
    transport: Transport,
    exit_status: Option<u32>,
    counter: Arc<Counter>,
}

/// struct that returns false for the first
/// n-1 callers and true for the nth caller
/// Used by setting the capacity to the number of multicaptures
#[derive(Debug)]
pub struct Counter {
    count: Mutex<u32>,
    capacity: u32,
}

impl Counter {
    pub fn new(capacity: u32) -> Arc<Self> {
        Arc::new(Self {
            count: Mutex::new(0),
            capacity,
        })
    }

    // increments count and returns true if count is equal to capacity
    pub fn call(&self) -> bool {
        let mut count = self.count.lock().unwrap();
        *count += 1;

        *count == self.capacity
    }
}

impl MultiCapture {
    /// Creates a new capture.
    pub fn new(endpoint: Endpoint, counter: Arc<Counter>) -> Self {
        Self {
            message: Default::default(),
            shutdown: Default::default(),
            endpoint,
            cur_count: RwLock::new(0),
            transport: Transport::Udp,
            exit_status: None,
            counter,
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

    // Set the exit status for capture to return with on shutdown
    pub fn exit_status(mut self, status: u32) -> Self {
        self.exit_status = Some(status);
        self
    }
}

#[async_trait::async_trait]
impl Protocol for MultiCapture {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let broadcast_endpoint = Endpoint::new(Ipv4Address::SUBNET, self.endpoint.port);

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
                    .listen(self.id(), self.endpoint, protocols.clone())
                    .unwrap();

                // listen on broadcast
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), broadcast_endpoint, protocols)
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

        if self.counter.call() {
            if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                if let Some(status) = self.exit_status {
                    shutdown.shut_down_with_status(ExitStatus::Status(status));
                } else {
                    // Exit with status: all machines that received the message, which is all of them since self.counter.call() -> true
                    shutdown.shut_down_with_status(ExitStatus::Status(self.counter.capacity));
                }
            }
        }

        Ok(())
    }
}
