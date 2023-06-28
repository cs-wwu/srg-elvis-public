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
pub struct WaitForMessage {
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    endpoint: Endpoint,
    /// The transport protocol to use
    transport: Transport,
    /// The message that was received, if any
    actual: RwLock<Message>,
    /// The message we expect to receive
    expected: Message,
    /// Whether to check that the bytes of the received message match. Turn on
    /// for tests and off for benchmarks.
    check_message: bool,
}

impl WaitForMessage {
    /// Creates a new capture.
    pub fn new(endpoint: Endpoint, expected: Message) -> Self {
        Self {
            endpoint,
            expected,
            transport: Transport::Udp,
            actual: Default::default(),
            shutdown: Default::default(),
            check_message: true,
        }
    }

    /// Set the transport protocol to use
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// Causes the received message bytes not to be checked against the expected
    /// message. Good for benchmarking.
    pub fn disable_checking(mut self) -> Self {
        self.check_message = false;
        self
    }
}

#[async_trait::async_trait]
impl Protocol for WaitForMessage {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
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
        let mut actual = self.actual.write().unwrap();
        actual.concatenate(message);

        if actual.len() < self.expected.len() {
            return Ok(());
        }

        if self.check_message {
            assert_eq!(self.expected, *actual);
        }

        if let Some(shutdown) = self.shutdown.write().unwrap().take() {
            shutdown.shut_down();
        }
        Ok(())
    }
}
