use super::dhcp_parsing::{DhcpMessage, MessageType};
use crate::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Endpoints, Udp},
    Control, Machine, Protocol, Session, Shutdown,
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
    pub state: RwLock<CurrentState>,
    lease: Option<RwLock<DelayQueue<LeaseRemaining>>>,
    l_time: u64,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address, l_time: u64) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
            state: RwLock::new(CurrentState::Init),
            lease: None,
            l_time,
        }
    }

    fn assign_time(&self) -> Option<RwLock<DelayQueue<LeaseRemaining>>> {
        if self.l_time > 0 {
            Some(RwLock::new(DelayQueue::new()))
        } else {
            None
        }
    }

    fn fill_lease(&self) {
        self.lease.as_ref().unwrap().write().unwrap().insert(LeaseRemaining::At50Percent, Duration::from_secs(self.l_time / 2));
        self.lease.as_ref().unwrap().write().unwrap().insert(LeaseRemaining::At25Percent, Duration::from_secs(self.l_time * 3 / 4));
        self.lease.as_ref().unwrap().write().unwrap().insert(LeaseRemaining::At0Percent, Duration::from_secs(self.l_time));
    }

    async fn lease_time(&self, caller: Arc<dyn Session>, machine: Arc<Machine>,) {
        self.fill_lease();
        println!("TEST");
        while !self.lease.as_ref().unwrap().read().unwrap().is_empty() {
            println!("2");
            //if client has shut down, clear the queue
            // if !shutdown.receiver().is_empty() {
            //     println!("Shutting down!");
            //     self.lease_shutdown();        
            // } else {
                let mut renew = DhcpMessage::default();
                let next = futures::future::poll_fn(|cx| self.lease.as_ref().unwrap().write().unwrap().poll_expired(cx)).await.unwrap().into_inner();
                match next {
                    //attempts to send another message to dhcp server for renewal
                    LeaseRemaining::At50Percent => {
                        println!("50 Percent Remaining!");
                        *self.state.write().unwrap() = CurrentState::Renewing.into();
                        renew.your_ip = self.ip_address().await;
                        renew.msg_type = MessageType::Request;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        caller.send(renew_message, machine.clone()).unwrap();
                        //println!("{:?}", *self.ip_address.read().unwrap());
                    }
                    //broadcasts a new discover message to find some other dhcp server for renewal
                    LeaseRemaining::At25Percent => {
                        println!("25 Percent Remaining!");
                        *self.state.write().unwrap() = CurrentState::Rebinding.into();
                        renew.your_ip = self.ip_address().await;
                        renew.msg_type = MessageType::Discover;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        caller.send(renew_message, machine.clone()).unwrap();
                    }
                    //removes ip and restarts dhcp process
                    LeaseRemaining::At0Percent => {
                        println!("All Done!");
                        *self.state.write().unwrap() = CurrentState::Init.into();
                        *self.ip_address.write().unwrap() = None;
                        renew.msg_type = MessageType::Discover;
                        let renew_message = DhcpMessage::to_message(renew).unwrap();
                        caller.send(renew_message, machine.clone()).unwrap();
                    }
                    LeaseRemaining::LeaseShutdown => {
                        //dummy item so that "next" doesn't get hung up with no more items in queue
                    }
                }
            // }
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
        self.lease.as_ref().unwrap().write().unwrap().clear();
        self.lease.as_ref().unwrap().write().unwrap().insert(LeaseRemaining::LeaseShutdown, Duration::from_secs(0));
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpClient {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
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
            //TO DO: Add arm for when Nack is received
            MessageType::Offer => {
                *self.state.write().unwrap() = CurrentState::Selecting.into();
                let mut response = DhcpMessage::default();
                response.your_ip = parsed_msg.your_ip;
                response.msg_type = MessageType::Request;
                response.op = 2;
                caller
                    .send(DhcpMessage::to_message(response).unwrap(), machine)
                    .unwrap();
                *self.state.write().unwrap() = CurrentState::Requesting.into();
                Ok(())
            }
            MessageType::Ack => {
                *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
                *self.state.write().unwrap() = CurrentState::Bound.into();
                self.notify.notify_waiters();
                self.assign_time();
                if self.lease.is_some() {
                    self.lease_time(caller, machine);
                }
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
