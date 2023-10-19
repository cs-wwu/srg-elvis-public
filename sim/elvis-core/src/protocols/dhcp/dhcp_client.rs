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
    LeaseShutdown,
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
    pub lease: RwLock<DelayQueue<LeaseRemaining>>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address, listen: Option<DhcpClientListener>) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
            listener: RwLock::new(listen),
            state: RwLock::new(CurrentState::Init),
            lease: RwLock::new(DelayQueue::new()),
        }
    }

    pub async fn ip_address(&self) -> Ipv4Address {
        if let Some(ip_address) = *self.ip_address.read().unwrap() {
            return ip_address;
        }
        self.notify.notified().await;
        self.ip_address.read().unwrap().unwrap()
    }

    pub fn lease_shutdown(&self) {
        self.lease.write().unwrap().clear();
        self.lease.write().unwrap().insert(LeaseRemaining::LeaseShutdown, Duration::from_secs(0));
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpClient {
    async fn start(
        &self,
        shutdown: Shutdown,
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

        //example at 8 second lease
        let time = 8;
        self.lease.write().unwrap().insert(LeaseRemaining::At50Percent, Duration::from_secs(time / 2));
        self.lease.write().unwrap().insert(LeaseRemaining::At25Percent, Duration::from_secs(time * 3 / 4));
        self.lease.write().unwrap().insert(LeaseRemaining::At0Percent, Duration::from_secs(time));

        while !self.lease.read().unwrap().is_empty() {
            //if client has shut down, clear the queue
            if !shutdown.receiver().is_empty() {
                println!("Shutting down!");
                self.lease_shutdown();        
            } else {
                let mut renew = DhcpMessage::default();
                let next = futures::future::poll_fn(|cx| self.lease.write().unwrap().poll_expired(cx)).await.unwrap().into_inner();
                if *self.state.read().unwrap() == CurrentState::Bound && self.lease.write().unwrap().len() > 0{
                    //reset the timer when the ip is rebound
                    self.lease.write().unwrap().clear();
                    println!("State is Bound, clearing queue");
                    self.lease.write().unwrap().insert(LeaseRemaining::At50Percent, Duration::from_secs(time / 2));
                    self.lease.write().unwrap().insert(LeaseRemaining::At25Percent, Duration::from_secs(time * 3 / 4));
                    self.lease.write().unwrap().insert(LeaseRemaining::At0Percent, Duration::from_secs(time));
                } else {
                    match next {
                        //attempts to send another message to dhcp server for renewal
                        LeaseRemaining::At50Percent => {
                            println!("50 Percent Remaining!");
                            *self.state.write().unwrap() = CurrentState::Renewing.into();
                            renew.your_ip = self.ip_address().await;
                            renew.msg_type = MessageType::Request;
                            let renew_message = DhcpMessage::to_message(renew).unwrap();
                            udp.send(renew_message, protocols.clone()).unwrap();
                            //println!("{:?}", *self.ip_address.read().unwrap());
                        }
                        //broadcasts a new discover message to find some other dhcp server for renewal
                        LeaseRemaining::At25Percent => {
                            println!("25 Percent Remaining!");
                            *self.state.write().unwrap() = CurrentState::Rebinding.into();
                            renew.your_ip = self.ip_address().await;
                            renew.msg_type = MessageType::Discover;
                            let renew_message = DhcpMessage::to_message(renew).unwrap();
                            udp.send(renew_message, protocols.clone()).unwrap();
                        }
                        //removes ip and restarts dhcp process
                        LeaseRemaining::At0Percent => {
                            println!("All Done!");
                            *self.state.write().unwrap() = CurrentState::Init.into();
                            *self.ip_address.write().unwrap() = None;
                            renew.msg_type = MessageType::Discover;
                            let renew_message = DhcpMessage::to_message(renew).unwrap();
                            udp.send(renew_message, protocols.clone()).unwrap();
                        }
                        LeaseRemaining::LeaseShutdown => {
                            //dummy item so that "next" doesn't get hung up with no more items in queue
                        }
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
                *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
                *self.state.write().unwrap() = CurrentState::Bound.into();
                self.notify.notify_waiters();
                if self.listener.read().unwrap().is_some(){
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
                    self.lease_shutdown();
                }
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
