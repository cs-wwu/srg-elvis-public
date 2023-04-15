use dashmap::{mapref::entry::Entry, DashMap};
use elvis_core::{
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    session::SharedSession,
    Control, Id, ProtocolMap, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

use super::Transport;

/// An application that sends a Time To Live (TTL) to
/// another machine from the first machine.
/// The second machine will then send the TTL back minus 1.
/// Once the TTL reaches 0 the program ends.
pub struct PingPongMulti {
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    client_count: RwLock<u8>,
    /// The session we send messages on
    sessions: RwLock<DashMap<Ipv4Address, SharedSession>>,
    is_initiator: bool,
    /// The address we listen for a message on
    local_ip_address: Ipv4Address,
    remote_ip_address_1: Ipv4Address,
    remote_ip_address_2: Ipv4Address,
    remote_ip_address_3: Ipv4Address,
    /// The port we listen for a message on
    local_port: u16,
    remote_port: u16,
    /// The machine that will receive the message
    remote_mac: Option<Mac>,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl PingPongMulti {
    /// Creates a new capture.
    pub fn new(
        is_initiator: bool,
        local_ip_address: Ipv4Address,
        remote_ip_address_1: Ipv4Address,
        remote_ip_address_2: Ipv4Address,
        remote_ip_address_3: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            is_initiator,
            shutdown: Default::default(),
            sessions: Default::default(),
            client_count: RwLock::new(3),
            local_ip_address,
            remote_ip_address_1,
            remote_ip_address_2,
            remote_ip_address_3,
            local_port,
            remote_port,
            remote_mac: None,
            transport: Transport::Udp,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
    /// Set the MAC address of the machine to send to
    pub fn remote_mac(mut self, mac: Mac) -> Self {
        self.remote_mac = Some(mac);
        self
    }
    /// The protocol to use in delivering the message
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for PingPongMulti {
    const ID: Id = Id::from_string("PingPongMulti");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);

        let mut participants_1 = Control::new();
        Ipv4::set_local_address(self.local_ip_address, &mut participants_1);
        Ipv4::set_remote_address(self.remote_ip_address_1, &mut participants_1);
        Udp::set_local_port(self.local_port, &mut participants_1);
        Udp::set_remote_port(self.remote_port, &mut participants_1);
        let protocol_1 = protocols.protocol(Udp::ID).expect("No such protocol");
        let session_1 = protocol_1.open(Self::ID, participants_1, protocols.clone())?;
        self.sessions.write().unwrap().insert(self.remote_ip_address_1, session_1.clone());

        if !self.is_initiator {
            let mut participants_2 = Control::new();
            Ipv4::set_local_address(self.local_ip_address, &mut participants_2);
            Ipv4::set_remote_address(self.remote_ip_address_2, &mut participants_2);
            Udp::set_local_port(self.local_port, &mut participants_2);
            Udp::set_remote_port(self.remote_port, &mut participants_2);
            let protocol_2 = protocols.protocol(Udp::ID).expect("No such protocol");
            let session_2 = protocol_2.open(Self::ID, participants_2, protocols.clone())?;
            self.sessions.write().unwrap().insert(self.remote_ip_address_2, session_2.clone());

            let mut participants_3 = Control::new();
            Ipv4::set_local_address(self.local_ip_address, &mut participants_3);
            Ipv4::set_remote_address(self.remote_ip_address_3, &mut participants_3);
            Udp::set_local_port(self.local_port, &mut participants_3);
            Udp::set_remote_port(self.remote_port, &mut participants_3);
            let protocol_3 = protocols.protocol(Udp::ID).expect("No such protocol");
            let session_3 = protocol_3.open(Self::ID, participants_3, protocols.clone())?;
            self.sessions.write().unwrap().insert(self.remote_ip_address_3, session_3.clone());
        }

        let context = Context::new(protocols);
        let is_initiator = self.is_initiator;
        tokio::spawn(async move {
            initialized.wait().await;
            if is_initiator {
                session_1
                    //Send the first "Ping" message with TTL of 255
                    .send(Message::new(vec![255]), context)
                    .unwrap();
            }
        });
        Ok(())
    }

    fn receive(&self, message: Message, context: Context) -> Result<(), ApplicationError> {
        let ttl = message.iter().next().expect("The message contained no TTL");

        if ttl % 2 == 0 {
            tracing::info!("Pong {}", ttl);
            //println!("Pong {}", ttl);
        } else {
            tracing::info!("Ping {}", ttl);
            //println!("Ping {}", ttl);
        }

        let ttl = ttl - 1;

        if ttl == 0 {
            tracing::info!("TTL has reach 0, PingPong has successfully completed");
            *self.client_count.write().unwrap() -= 1;
            if *self.client_count.read().unwrap() <= 0 {
                if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                    shutdown.shut_down();
                }
            }
        } else {
            let remote_ip_address = Ipv4::get_remote_address(&context.control).unwrap();
            match self.sessions.read().unwrap().entry(remote_ip_address) {
                Entry::Occupied(entry) => {
                    entry.get().clone().send(Message::new(vec![ttl]), context)?;
                }
                Entry::Vacant(_) => {
                    return Err(ApplicationError::Other)
                }
            }
        }
        Ok(())
    }
}
