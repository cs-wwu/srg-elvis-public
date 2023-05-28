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

use super::dhcp::dhcp_client::DhcpClient;

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
        let messages = std::mem::take(&mut *self.messages.write().unwrap());
        let endpoint = self.endpoint;
        let transport = self.transport;
        tokio::spawn(async move {
            initialized.wait().await;

            let local_address = match protocols.protocol::<UserProcess<DhcpClient>>() {
                Some(dhcp) => dhcp.application().ip_address().await,
                None => Ipv4Address::LOCALHOST,
            };

            println!("Got local address {local_address}");

            let endpoints = Endpoints {
                local: Endpoint {
                    address: local_address,
                    port: 0,
                },
                remote: endpoint,
            };

            let session = match transport {
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
                    .open_for_sending(
                        TypeId::of::<UserProcess<Self>>(),
                        endpoints,
                        protocols.clone(),
                    )
                    .unwrap(),
            };

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
