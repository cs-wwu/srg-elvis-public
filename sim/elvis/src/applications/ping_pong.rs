use super::Transport;
use elvis_core::{
    gcd::{self, get_protocol},
    message::Message,
    network::Mac,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    session::SharedSession,
    Control, Id,
};
use std::sync::{Arc, RwLock};

/// An application that sends a Time To Live (TTL) to
/// another machine from the first machine.
/// The second machine will then send the TTL back minus 1.
/// Once the TTL reaches 0 the program ends.
pub struct PingPong {
    /// The session we send messages on
    session: RwLock<Option<SharedSession>>,
    is_initiator: bool,
    /// The address we listen for a message on
    local_ip_address: Ipv4Address,
    remote_ip_address: Ipv4Address,
    /// The port we listen for a message on
    local_port: u16,
    remote_port: u16,
    /// The machine that will receive the message
    remote_mac: Option<Mac>,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl PingPong {
    /// Creates a new capture.
    pub fn new(
        is_initiator: bool,
        local_ip_address: Ipv4Address,
        remote_ip_address: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            is_initiator,
            session: Default::default(),
            local_ip_address,
            remote_ip_address,
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

impl Application for PingPong {
    const ID: Id = Id::from_string("PingPong");

    fn start(&self) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(self.local_ip_address, &mut participants);
        Ipv4::set_remote_address(self.remote_ip_address, &mut participants);
        Udp::set_local_port(self.local_port, &mut participants);
        Udp::set_remote_port(self.remote_port, &mut participants);
        let protocol = get_protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants)?;
        *self.session.write().unwrap() = Some(session.clone());

        let is_initiator = self.is_initiator;
        gcd::job(move || {
            if is_initiator {
                session
                    //Send the first "Ping" message with TTL of 255
                    .send(Message::new(vec![255]), Control::new())
                    .unwrap();
            }
        });
        Ok(())
    }

    fn receive(&self, message: Message, control: Control) -> Result<(), ApplicationError> {
        let ttl = message.iter().next().expect("The message contained no TTL");

        if ttl % 2 == 0 {
            eprintln!("Pong {}", ttl);
        } else {
            eprintln!("Ping {}", ttl);
        }

        let ttl = ttl - 1;

        if ttl == 0 {
            eprintln!("TTL has reach 0, PingPong has successfully completed");
            gcd::shut_down();
        } else {
            self.session
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .send(Message::new(vec![ttl]), control)?;
        }
        Ok(())
    }
}
