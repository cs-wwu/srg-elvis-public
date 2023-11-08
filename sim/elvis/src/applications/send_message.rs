use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{dhcp::dhcp_client::DhcpClient, ipv4::Ipv4Address, Endpoint, Endpoints, Tcp, Udp},
    Control, Protocol, Session, Shutdown, Transport,
};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::Barrier;

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The body of the message to send
    messages: RwLock<Vec<Message>>,
    endpoints: Vec<Endpoint>,
    /// The protocol to use in delivering the message
    transport: Transport,
    /// the application's local address
    local_ip: Ipv4Address,
    delay: Option<Duration>,
}

impl SendMessage {
    /// Creates a new send message application that sends a message to a single machine.
    pub fn new(messages: Vec<Message>, endpoint: Endpoint) -> Self {
        Self {
            messages: RwLock::new(messages),
            endpoints: Vec::from([endpoint]),
            transport: Transport::Udp,
            local_ip: Ipv4Address::LOCALHOST,
            delay: None,
        }
    }

    /// Creates a new send message application that sends a message to multiple machines.
    pub fn with_endpoints(messages: Vec<Message>, endpoints: Vec<Endpoint>) -> Self {
        Self {
            messages: RwLock::new(messages),
            endpoints,
            transport: Transport::Udp,
            local_ip: Ipv4Address::LOCALHOST,
            delay: None,
        }
    }

    /// Set the local IP address of this protocol.
    /// (By default, its local IP is `127.0.0.1`)
    pub fn local_ip(mut self, local_ip: Ipv4Address) -> Self {
        self.local_ip = local_ip;
        self
    }

    /// The protocol to use in delivering the message
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    pub fn delay(mut self, duration: Duration) -> Self {
        self.delay = Some(duration);
        self
    }
}

#[async_trait::async_trait]
impl Protocol for SendMessage {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let messages = std::mem::take(&mut *self.messages.write().unwrap());
        let endpoints = &self.endpoints;
        let transport = self.transport;
        initialized.wait().await;

        let local_address = match protocols.protocol::<DhcpClient>() {
            Some(dhcp) => dhcp.ip_address().await,
            None => self.local_ip,
        };

        if let Some(duration) = self.delay {
            tokio::time::sleep(duration).await;
        }

        for endpoint in endpoints.iter() {
            let endpoints = Endpoints {
                local: Endpoint {
                    address: local_address,
                    port: 0,
                },
                remote: *endpoint,
            };

            let session = match transport {
                Transport::Tcp => protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .open(self.id(), endpoints, protocols.clone())
                    .await
                    .unwrap(),
                Transport::Udp => protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .open_for_sending(self.id(), endpoints, protocols.clone())
                    .await
                    .unwrap(),
            };
            
            println!("Sending message");

            for message in messages.iter() {
                session
                    .send(message.clone(), protocols.clone())
                    .expect("SendMessage failed to send");
            }
        }
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}
