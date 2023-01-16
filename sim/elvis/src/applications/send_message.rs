use elvis_core::{
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4,
    },
    Control, Id, Network, ProtocolMap,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The text of the message to send
    text: &'static str,
    /// The IP address to send to
    ip: Ipv4Address,
    /// The port to send on
    port: u16,
    /// The machine that will receive the message
    destination_mac: Option<Mac>,
    count: u16,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(
        text: &'static str,
        remote_ip: Ipv4Address,
        remote_port: u16,
        destination_mac: Option<Mac>,
        count: u16,
    ) -> Self {
        Self {
            text,
            ip: remote_ip,
            port: remote_port,
            destination_mac,
            count,
        }
    }

    /// Creates a new send message application behind a shared handle.
    pub fn new_shared(
        text: &'static str,
        remote_ip: Ipv4Address,
        remote_port: u16,
        destination_mac: Option<Mac>,
        count: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            text,
            remote_ip,
            remote_port,
            destination_mac,
            count,
        ))
    }
}

impl Application for SendMessage {
    const ID: Id = Id::from_string("Send Message");

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(Ipv4Address::LOCALHOST, &mut participants);
        Ipv4::set_remote_address(self.ip, &mut participants);
        Udp::set_local_port(0, &mut participants);
        Udp::set_remote_port(self.port, &mut participants);
        let protocol = protocols.protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants, protocols.clone())?;
        let mut context = Context::new(protocols);
        tokio::spawn(async move {
            initialized.wait().await;
            if let Some(destination_mac) = self.destination_mac {
                Network::set_destination(destination_mac, &mut context.control);
            }
            for _ in 0..self.count {
                session
                    .clone()
                    .send(Message::new(self.text), context.clone())
                    .expect("SendMessage failed to send");
            }
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
