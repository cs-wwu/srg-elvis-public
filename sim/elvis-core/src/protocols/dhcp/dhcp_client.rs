use super::dhcp_parsing::{DhcpMessage, MessageType};
use crate::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Endpoints, Udp},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{Barrier, Notify};

#[derive(Default)]
pub struct DhcpClient {
    server_ip: Ipv4Address,
    notify: Arc<Notify>,
    pub ip_address: RwLock<Option<Ipv4Address>>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
        }
    }

    pub async fn ip_address(&self) -> Ipv4Address {
        if let Some(ip_address) = *self.ip_address.read().unwrap() {
            return ip_address;
        }
        self.notify.notified().await;
        self.ip_address.read().unwrap().unwrap()
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: DoneSender,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        let server_ip = self.server_ip;
        let endpoints = Endpoints {
            local: Endpoint {
                address: Ipv4Address::new([0, 0, 0, 0]),
                port: 68,
            },
            remote: Endpoint {
                address: server_ip,
                port: 67,
            },
        };
        let udp = machine.protocol::<Udp>().unwrap();
        udp.listen(self.id(), endpoints.local, machine.clone())
            .unwrap();

        // Wait on initialization before sending any message across the network
        initialized.wait().await;

        let udp_session = udp
            .open_for_sending(self.id(), endpoints, machine.clone())
            .await
            .unwrap();

        let response = DhcpMessage::default();
        let response_message = DhcpMessage::to_message(response).unwrap();
        udp_session.send(response_message, machine).unwrap();

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        let parsed_msg = DhcpMessage::from_bytes(message.iter()).unwrap();
        match parsed_msg.msg_type {
            MessageType::Offer => {
                let mut response = DhcpMessage::default();
                response.your_ip = parsed_msg.your_ip;
                response.msg_type = MessageType::Request;
                response.op = 2;
                caller
                    .send(DhcpMessage::to_message(response).unwrap(), machine)
                    .unwrap();
                Ok(())
            }
            MessageType::Ack => {
                *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
                self.notify.notify_waiters();
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
