use elvis_core::{
    gcd::GcdHandle,
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    Control, Id, ProtocolMap,
};
use std::sync::{Arc, RwLock};

use super::Transport;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct WaitForMessage {
    /// The channel we send on to shut down the simulation
    gcd: RwLock<Option<GcdHandle>>,
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
            gcd: Default::default(),
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

    fn start(&self, gcd: GcdHandle, protocols: ProtocolMap) -> Result<(), ApplicationError> {
        *self.gcd.write().unwrap() = Some(gcd);
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);

        Udp::set_local_port(self.port, &mut participants);
        // match self.transport {
        //     Transport::Udp => Udp::set_local_port(self.port, &mut participants),
        //     Transport::Tcp => Tcp::set_local_port(self.port, &mut participants),
        // }

        protocols
            .protocol(self.transport.id())
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)?;
        Ok(())
    }

    fn receive(&self, message: Message, _context: Context) -> Result<(), ApplicationError> {
        println!("Receive");
        let mut actual = self.actual.write().unwrap();
        actual.concatenate(message);

        if actual.len() < self.expected.len() {
            return Ok(());
        }

        if self.check_message {
            assert_eq!(self.expected, *actual);
        }

        if let Some(gcd) = self.gcd.write().unwrap().take() {
            gcd.shut_down();
        }
        Ok(())
    }
}
