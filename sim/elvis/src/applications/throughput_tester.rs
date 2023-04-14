use elvis_core::{
    gcd::{self, get_protocol},
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    Control, Id,
};
use std::{
    ops::Range,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct ThroughputTester {
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
    message_count: u8,
    expected_delay: Range<Duration>,
    previous_receipt: Arc<RwLock<Option<SystemTime>>>,
    received: Arc<RwLock<u8>>,
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
            ip_address,
            port,
            message_count,
            expected_delay,
            previous_receipt: Arc::new(RwLock::new(None)),
            received: Arc::new(RwLock::new(0)),
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for ThroughputTester {
    const ID: Id = Id::from_string("Capture");

    fn start(&self) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);
        Udp::set_local_port(self.port, &mut participants);
        get_protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants)?;
        Ok(())
    }

    fn receive(&self, _message: Message, _control: Control) -> Result<(), ApplicationError> {
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
            gcd::shut_down();
        }
        Ok(())
    }
}
