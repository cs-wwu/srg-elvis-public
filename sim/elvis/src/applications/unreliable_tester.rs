use elvis_core::{
    protocol::Context,
    protocols::{
        user_process::{Application, ApplicationError},
        Ipv4, Udp, UserProcess,
    },
    Control, Id, Message,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::sync::{mpsc::Sender, Barrier};

/// An application used for testing the
/// [`Unreliable`](crate::networks::Unreliable) network type.
///
/// It sends 100 messages to another machine and counts how many responses it
/// receives in return. The other machine will be running a
/// [`Forward`](super::Forward) program that just sends back any messages sent
/// to it.
pub struct UnreliableTester {
    /// The channel for ending the simulation
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    /// The time the most recent message was received
    last_receipt: Arc<Mutex<SystemTime>>,
    /// How many messages have been received
    receipt_count: Arc<Mutex<u16>>,
}

impl UnreliableTester {
    /// Creates a new instance of the application
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the application.
    pub fn new_shared() -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new())
    }

    /// How many messages were received back in response.
    pub fn receipt_count(self: Arc<Self>) -> u16 {
        *self.receipt_count.lock().unwrap()
    }
}

impl Default for UnreliableTester {
    fn default() -> Self {
        Self {
            shutdown: Default::default(),
            last_receipt: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
            receipt_count: Default::default(),
        }
    }
}

impl Application for UnreliableTester {
    const ID: Id = Id::from_string("Unreliable tester");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), ApplicationError> {
        // Synchronous initialization
        *self.shutdown.lock().unwrap() = Some(shutdown);
        *self.last_receipt.lock().unwrap() = SystemTime::now();
        let mut participants = Control::new();
        Ipv4::set_local_address([0, 0, 0, 0].into(), &mut participants);
        Ipv4::set_remote_address([0, 0, 0, 1].into(), &mut participants);
        Udp::set_local_port(0xdead, &mut participants);
        Udp::set_remote_port(0xdead, &mut participants);
        let udp = context.protocol(Udp::ID).expect("No such protocol");
        udp.clone()
            .listen(Self::ID, participants.clone(), context.clone())?;
        let send_session = udp.clone().open(Self::ID, participants, context.clone())?;

        tokio::spawn(async move {
            initialized.wait().await;
            tokio::spawn(async move {
                // Repeatedly wait for five milliseconds until the most recently
                // received message came in at least twenty-five milliseconds
                // ago. When we haven't seen a message for a while, it indicates
                // that all messages have either been delivered or been lost in
                // transit and it's time to shut things down.
                loop {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    let now = SystemTime::now();
                    let then = *self.last_receipt.lock().unwrap();
                    let duration = now.duration_since(then).unwrap();
                    if duration > Duration::from_millis(25) {
                        tokio::spawn(async move {
                            let shutdown = self.shutdown.lock().unwrap().as_ref().unwrap().clone();
                            shutdown.send(()).await.unwrap()
                        });
                        break;
                    }
                }
            });

            // Send 100 messages to our peer
            for i in 0..100u32 {
                send_session
                    .clone()
                    .send(Message::new(&i.to_be_bytes()), context.clone())
                    .expect("UnreliableTester failed to send");
            }
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        *self.last_receipt.lock().unwrap() = SystemTime::now();
        *self.receipt_count.lock().unwrap() += 1;
        Ok(())
    }
}
