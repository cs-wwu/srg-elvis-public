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
use tokio_util::time::{DelayQueue};

pub enum LeaseRemaining {
    At50Percent,
    At25Percent,
    At0Percent,
}

#[derive(Default, PartialEq, Debug)]
pub enum CurrentState {
    #[default]
    Init,
    Selecting,
    Requesting,
    InitReboot,
    Rebooting,
    Bound,
    Renewing,
    Rebinding,
}

#[derive(Default)]
pub struct DhcpClient {
    server_ip: Ipv4Address,
    notify: Arc<Notify>,
    pub ip_address: RwLock<Option<Ipv4Address>>,
    listener: RwLock<Option<DhcpClientListener>>,
    pub state: RwLock<CurrentState>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address, listen: Option<DhcpClientListener>) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
            listener: RwLock::new(listen),
            state: RwLock::new(CurrentState::Init),
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
        udp.send(response_message, protocols.clone()).unwrap();

        //TO DO: implement something to ensure DelayQueue is not starting until ip is assigned

        let mut delay_queue = DelayQueue::new();

        //example at 5 second lease
        let time = 5;
        delay_queue.insert(LeaseRemaining::At50Percent, Duration::from_secs(time / 2));
        delay_queue.insert(LeaseRemaining::At25Percent, Duration::from_secs(time * 3 / 4));
        delay_queue.insert(LeaseRemaining::At0Percent, Duration::from_secs(time));

        while !delay_queue.is_empty() {
            //if client has shut down, clear the queue
            if *self.ip_address.read().unwrap() == None {
                println!("AAAAAAAAAAAAAAAA");
                delay_queue.clear();                
            } else {
                if *self.state.read().unwrap() == CurrentState::Bound {
                //reset the timer when the ip is rebound
                    *self.state.write().unwrap() = CurrentState::Rebinding.into();
                    delay_queue.clear();
                    delay_queue.insert(LeaseRemaining::At50Percent, Duration::from_secs(time / 2));
                    delay_queue.insert(LeaseRemaining::At25Percent, Duration::from_secs(time * 3 / 4));
                    delay_queue.insert(LeaseRemaining::At0Percent, Duration::from_secs(time));
                }
                let next = futures::future::poll_fn(|cx| delay_queue.poll_expired(cx)).await.unwrap().into_inner();
                match next {
                    //attempts to send another message to dhcp server for renewal
                    LeaseRemaining::At50Percent => {
                        *self.state.write().unwrap() = CurrentState::Renewing.into();
                        let mut renew = DhcpMessage::default();
                        renew.your_ip = self.ip_address().await;
                        renew.msg_type = MessageType::Request;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        udp.send(renew_message, protocols.clone()).unwrap();
                        println!("50 Percent Remaining!");
                    }
                    //broadcasts a new discover message to find some other dhcp server for renewal
                    LeaseRemaining::At25Percent => {
                        *self.state.write().unwrap() = CurrentState::Rebinding.into();
                        let mut renew = DhcpMessage::default();
                        renew.your_ip = self.ip_address().await;
                        renew.msg_type = MessageType::Discover;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        udp.send(renew_message, protocols.clone()).unwrap();
                        println!("25 Percent Remaining!");
                    }
                    //removes ip and restarts dhcp process
                    LeaseRemaining::At0Percent => {
                        *self.state.write().unwrap() = CurrentState::Init.into();
                        *self.ip_address.write().unwrap() = None;
                        let mut renew = DhcpMessage::default();
                        renew.msg_type = MessageType::Discover;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        udp.send(renew_message, protocols.clone()).unwrap();
                        println!("All Done!");
                    }
                }
            }
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
                *self.state.write().unwrap() = CurrentState::Selecting.into();
                let mut response = DhcpMessage::default();
                response.your_ip = parsed_msg.your_ip;
                response.msg_type = MessageType::Request;
                response.op = 2;
                caller
                    .send(DhcpMessage::to_message(response).unwrap(), protocols)
                    .unwrap();
                *self.state.write().unwrap() = CurrentState::Requesting.into();
                Ok(())
            }
            MessageType::Ack => {
                let mut first = false;
                if *self.ip_address.read().unwrap() == Some(Ipv4Address::new([0,0,0,0])) {
                    first = true;
                }
                *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
                *self.state.write().unwrap() = CurrentState::Bound.into();
                self.notify.notify_waiters();
                if self.listener.read().unwrap().is_some() && first == true {
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
