use elvis_core::{
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
use tokio::sync::{mpsc::Sender, Barrier};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug, Clone)]
pub struct Capture {
    /// The message that was received, if any
    message: Arc<RwLock<Option<Message>>>,
    /// The channel we send on to shut down the simulation
    shutdown: Arc<RwLock<Option<Sender<()>>>>,
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
        self.message.read().unwrap().clone()
    }
}

impl Application for Capture {
    const ID: Id = Id::from_string("Capture");

    fn start(
        self: Arc<Self>,
        shutdown: Sender<()>,
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
        message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        *self.message.write().unwrap() = Some(message);
        if let Some(shutdown) = self.shutdown.write().unwrap().take() {
            tokio::spawn(async move {
                shutdown.send(()).await.unwrap();
            });
        }
        Ok(())
    }
}
