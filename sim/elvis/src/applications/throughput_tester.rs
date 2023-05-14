use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Udp,
    },
    Control, Participants, Protocol, Shutdown,
};
use std::{
    any::TypeId,
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
            shutdown: Default::default(),
            ip_address,
            port,
            message_count,
            expected_delay,
            previous_receipt: Arc::new(RwLock::new(None)),
            received: Arc::new(RwLock::new(0)),
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

impl Application for ThroughputTester {
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        let mut participants = Participants::new();
        participants.local.address = Some(self.ip_address);
        participants.local.port = Some(self.port);
        protocols
            .protocol::<Udp>()
            .expect("No such protocol")
            .listen(TypeId::of::<Self>(), participants, protocols)?;
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        &self,
        _message: Message,
        _control: Control,
        _protocols: ProtocolMap,
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
            if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                shutdown.shut_down();
            }
        }
        Ok(())
    }
}
