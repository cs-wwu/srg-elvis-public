use elvis_core::{
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Tcp,
    },
    Control, Id, Network, ProtocolMap,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

use super::Transport;

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The body of the message to send
    body: Message,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send on
    remote_port: u16,
    /// The machine that will receive the message
    remote_mac: Option<Mac>,
    /// The number of copies of the message to send
    count: u32,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(body: Message, remote_ip: Ipv4Address, remote_port: u16) -> Self {
        Self {
            body,
            remote_ip,
            remote_port,
            remote_mac: None,
            count: 1,
            transport: Transport::Udp,
        }
    }

    /// Wrap the SendMessage in a user process
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    /// Set the MAC address of the machine to send to
    pub fn remote_mac(mut self, mac: Mac) -> Self {
        self.remote_mac = Some(mac);
        self
    }

    /// The number of copies of the message to send
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// The protocol to use in delivering the message
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for SendMessage {
    const ID: Id = Id::from_string("Send Message");

    fn start(
        &self,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(Ipv4Address::LOCALHOST, &mut participants);
        Ipv4::set_remote_address(self.remote_ip, &mut participants);
        match self.transport {
            Transport::Udp => {
                Udp::set_local_port(0, &mut participants);
                Udp::set_remote_port(self.remote_port, &mut participants);
            }
            Transport::Tcp => {
                Tcp::set_local_port(0, &mut participants);
                Tcp::set_remote_port(self.remote_port, &mut participants);
            }
        }
        let protocol = protocols
            .protocol(self.transport.id())
            .expect("No such protocol");
        let session = protocol.open(Self::ID, participants, protocols.clone())?;
        let mut context = Context::new(protocols);

        let remote_mac = self.remote_mac;
        let count = self.count;
        let body = self.body.clone();
        tokio::spawn(async move {
            initialized.wait().await;
            if let Some(destination_mac) = remote_mac {
                Network::set_destination(destination_mac, &mut context.control);
            }
            for _ in 0..count {
                session
                    .clone()
                    .send(body.clone(), context.clone())
                    .expect("SendMessage failed to send");
            }
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
