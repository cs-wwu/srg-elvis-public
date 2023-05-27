use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Endpoint, Endpoints, Tcp, Udp,
    },
    Control, Session, Shutdown, Transport,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The body of the message to send
    messages: RwLock<Vec<Message>>,
    endpoint: Endpoint,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(messages: Vec<Message>, endpoint: Endpoint) -> Self {
        Self {
            messages: RwLock::new(messages),
            endpoint,
            transport: Transport::Udp,
        }
    }

    /// Wrap the SendMessage in a user process
    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }

    /// The protocol to use in delivering the message
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for SendMessage {
    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let endpoints = Endpoints {
            local: Endpoint {
                address: Ipv4Address::LOCALHOST,
                port: 0,
            },
            remote: self.endpoint,
        };

        let session = match self.transport {
            Transport::Tcp => protocols
                .protocol::<Tcp>()
                .unwrap()
                .open(
                    TypeId::of::<UserProcess<Self>>(),
                    endpoints,
                    protocols.clone(),
                )
                .unwrap(),
            Transport::Udp => protocols
                .protocol::<Udp>()
                .unwrap()
                .open(
                    TypeId::of::<UserProcess<Self>>(),
                    endpoints,
                    protocols.clone(),
                )
                .unwrap(),
        };

        let messages = std::mem::take(&mut *self.messages.write().unwrap());
        tokio::spawn(async move {
            initialized.wait().await;
            for message in messages {
                session
                    .send(message, protocols.clone())
                    .expect("SendMessage failed to send");
            }
        });
        Ok(())
    }

    fn receive(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
