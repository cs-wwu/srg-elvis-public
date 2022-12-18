use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    Control, Id,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::Sender, Barrier};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug, Clone)]
pub struct Capture {
    /// The message that was received, if any
    message: Arc<Mutex<Option<Message>>>,
    /// The channel we send on to shut down the simulation
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(ip_address: Ipv4Address, port: u16) -> Self {
        Self {
            message: Default::default(),
            shutdown: Default::default(),
            ip_address,
            port,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(ip_address: Ipv4Address, port: u16) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(ip_address, port))
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.lock().unwrap().clone()
    }
}

impl Application for Capture {
    const ID: Id = Id::from_string("Capture");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.lock().unwrap() = Some(shutdown);
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);
        Udp::set_local_port(self.port, &mut participants);
        context
            .protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, context)?;
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        *self.message.lock().unwrap() = Some(message);
        if let Some(shutdown) = self.shutdown.lock().unwrap().take() {
            tokio::spawn(async move {
                shutdown.send(()).await.unwrap();
            });
        }
        Ok(())
    }
}
