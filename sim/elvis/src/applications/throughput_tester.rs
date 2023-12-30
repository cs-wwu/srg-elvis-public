use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, Udp},
    Control, Machine, Protocol, Session, Shutdown,
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
    endpoint: Endpoint,
    message_count: u8,
    expected_delay: Range<Duration>,
    previous_receipt: Arc<RwLock<Option<SystemTime>>>,
    received: Arc<RwLock<u8>>,
}

impl ThroughputTester {
    /// Creates a new capture.
    pub fn new(endpoint: Endpoint, message_count: u8, expected_delay: Range<Duration>) -> Self {
        Self {
            shutdown: Default::default(),
            endpoint,
            message_count,
            expected_delay,
            previous_receipt: Arc::new(RwLock::new(None)),
            received: Arc::new(RwLock::new(0)),
        }
    }
}

impl Protocol for ThroughputTester {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        machine
            .protocol::<Udp>()
            .expect("No such protocol")
            .listen(self.id(), self.endpoint, machine)
            .unwrap();

        initialized.wait().await;

        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
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
