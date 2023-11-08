use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Tcp, Udp},
    shutdown::ExitStatus,
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
    exit_status: Option<Arc<RwLock<u32>>>,
    /// Increment the status reference instead of shutting down the sim
    status_increment: Option<u32>,
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
            exit_status: None,
            status_increment: None,
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

    /// Set the exit status for capture to return with on shutdown
    pub fn exit_status(mut self, status: u32) -> Self {
        self.exit_status = Some(Arc::new(RwLock::new(status)));
        self
    }

    /// Instead of exiting with a specific status after capturing a message, increment the status pointer by a certain amount
    pub fn with_atomic_status(mut self, status: Arc<RwLock<u32>>, increment: u32) -> Self {
        self.exit_status = Some(status);
        self.status_increment = Some(increment);
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
        
        println!("Recieved message");

        if *self.cur_count.read().unwrap() >= self.message_count {
            if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                if let Some(status_inc) = self.status_increment {
                    let mut status_ref = self
                        .exit_status
                        .as_ref()
                        .expect("If self.status_increment is Some, exit_status should also be Some")
                        .write()
                        .unwrap();
                    println!("incrementing status ref");
                    *status_ref += status_inc;
                } else {
                    // Shutdown the simulation
                    if let Some(status) = self.exit_status.clone() {
                        shutdown.shut_down_with_status(ExitStatus::Status(*status.read().unwrap()));
                    } else {
                        shutdown.shut_down();
                    }
                }
            }
        }
        Ok(())
    }
}
