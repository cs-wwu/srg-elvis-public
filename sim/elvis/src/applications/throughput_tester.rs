use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    Control, Id, ProtocolMap, Shutdown,
};
use std::{
    ops::Range,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::sync::Barrier;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct ThroughputTester {
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
    message_count: u8,
    expected_delay: Range<Duration>,
    previous_receipt: RwLock<Option<SystemTime>>,
    received: RwLock<u8>,
}

impl ThroughputTester {
    /// Creates a new capture.
    pub fn new(
        ip_address: Ipv4Address,
        port: u16,
        message_count: u8,
        expected_delay: Range<Duration>,
    ) -> Self {
        Self {
            shutdown: Default::default(),
            ip_address,
            port,
            message_count,
            expected_delay,
            previous_receipt: RwLock::new(None),
            received: RwLock::new(0),
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(
        ip_address: Ipv4Address,
        port: u16,
        message_count: u8,
        expected_delay: Range<Duration>,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(ip_address, port, message_count, expected_delay))
    }
}

impl Application for ThroughputTester {
    const ID: Id = Id::from_string("Capture");

    fn start(
        self: Arc<Self>,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);
        Udp::set_local_port(self.port, &mut participants);
        protocols
            .protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)?;
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        let now = SystemTime::now();
        if let Some(previous) = self.previous_receipt.write().unwrap().replace(now) {
            let elapsed = now.duration_since(previous).unwrap();
            assert!(self.expected_delay.contains(&elapsed));
        }

        let received = {
            let mut received = self.received.write().unwrap();
            *received += 1;
            *received
        };

        if received >= self.message_count {
            self.shutdown.write().unwrap().take().unwrap().shut_down();
        }
        Ok(())
    }
}
