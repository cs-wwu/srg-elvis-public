use elvis_core::{
    machine::ProtocolMap,
    message::Message,
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
pub struct WaitForMessage {
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
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
    pub fn new(ip_address: Ipv4Address, port: u16, expected: Message) -> Self {
        Self {
            ip_address,
            port,
            expected,
            transport: Transport::Udp,
            actual: Default::default(),
            shutdown: Default::default(),
            check_message: true,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
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

impl Application for WaitForMessage {
    const ID: Id = Id::from_string("Wait for message");

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

    fn receive(
        &self,
        message: Message,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
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
