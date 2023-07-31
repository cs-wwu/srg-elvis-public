use super::dhcp_client_listener::DhcpClientListener;
use super::dhcp_parsing::{DhcpMessage, MessageType};
use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Endpoints, Udp},
    Control, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{Barrier, Notify};
use tokio::time::Duration;
use tokio_util::time::{DelayQueue, delay_queue};

#[derive(Default)]
pub struct DhcpClient {
    server_ip: Ipv4Address,
    notify: Arc<Notify>,
    pub ip_address: RwLock<Option<Ipv4Address>>,
    listener: RwLock<Option<DhcpClientListener>>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address, listen: Option<DhcpClientListener>) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
            listener: RwLock::new(listen),
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
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let server_ip = self.server_ip;
        // Wait on initialization before sending any message across the network

        initialized.wait().await;
        let sockets = Endpoints {
            local: Endpoint {
                address: Ipv4Address::new([0, 0, 0, 0]),
                port: 68,
            },
            remote: Endpoint {
                address: server_ip,
                port: 67,
            },
        };
        let udp = protocols
            .protocol::<Udp>()
            .unwrap()
            .open_and_listen(self.id(), sockets, protocols.clone())
            .await
            .unwrap();

        let response = DhcpMessage::default();
        let response_message = DhcpMessage::to_message(response).unwrap();
        udp.send(response_message, protocols).unwrap();

        //TO DO: implement something to ensure DelayQueue is not starting until ip is assigned

        let mut delay_queue = DelayQueue::new();

        delay_queue.insert("timetest1", Duration::from_secs(2));
        delay_queue.insert("timetest2", Duration::from_secs(4));

        while !delay_queue.is_empty() {
            let next = futures::future::poll_fn(|cx| delay_queue.poll_expired(cx)).await;
            println!("{:?}", next.unwrap().into_inner());
            println!("{:?}", self.ip_address);
        }

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let parsed_msg = DhcpMessage::from_bytes(message.iter()).unwrap();
        match parsed_msg.msg_type {
            //TO DO: Add arm for when Nack is received
            MessageType::Offer => {
                let mut response = DhcpMessage::default();
                response.your_ip = parsed_msg.your_ip;
                response.msg_type = MessageType::Request;
                response.op = 2;
                caller
                    .send(DhcpMessage::to_message(response).unwrap(), protocols)
                    .unwrap();
                Ok(())
            }
            MessageType::Ack => {
                *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
                self.notify.notify_waiters();
                if self.listener.read().unwrap().is_some() {
                    if let Some(release) = self
                        .listener
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .update(parsed_msg.your_ip)
                    {
                        caller
                            .send(DhcpMessage::to_message(release).unwrap(), protocols.clone())
                            .unwrap();
                        *self.ip_address.write().unwrap() = None;
                        caller
                            .send(
                                DhcpMessage::to_message(DhcpMessage::default()).unwrap(),
                                protocols,
                            )
                            .unwrap();
                    }
                }
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
