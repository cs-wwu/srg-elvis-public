use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
    },
    Control, Id, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

use super::Transport;

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The body of the message to send
    messages: RwLock<Vec<Message>>,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send on
    remote_port: u16,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(messages: Vec<Message>, remote_ip: Ipv4Address, remote_port: u16) -> Self {
        Self {
            messages: RwLock::new(messages),
            remote_ip,
            remote_port,
            transport: Transport::Udp,
        }
    }

    /// Wrap the SendMessage in a user process
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    /// The protocol to use in delivering the message
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for SendMessage {
    const ID: Id = Id::from_string("Send Message");

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        participants.local.address = Some(Ipv4Address::LOCALHOST);
        participants.remote.address = Some(self.remote_ip);
        participants.local.port = Some(0);
        participants.remote.port = Some(self.remote_port);
        let protocol = protocols
            .protocol(self.transport.id())
            .expect("No such protocol");
        let session = protocol.open(Self::ID, participants, protocols.clone())?;
        let context = Context::new(protocols);
        let messages = std::mem::take(&mut *self.messages.write().unwrap());
        tokio::spawn(async move {
            initialized.wait().await;
            for message in messages {
                session
                    .send(message, context.clone())
                    .expect("SendMessage failed to send");
            }
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
