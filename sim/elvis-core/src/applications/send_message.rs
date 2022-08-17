use crate::{
    core::{
        message::Message,
        protocol::{Context, ProtocolId},
        Control,
    },
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{error::Error, sync::Arc};
use tokio::sync::mpsc::Sender;

/// An application that sends a single message over the network.
pub struct SendMessage {
    /// The text of the message to send
    text: &'static str,
    /// The IP address to send to
    ip: Ipv4Address,
    /// The port to send on
    port: u16,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(text: &'static str, remote_ip: Ipv4Address, remote_port: u16) -> Self {
        Self {
            text,
            ip: remote_ip,
            port: remote_port,
        }
    }

    /// Creates a new send message application behind a shared handle.
    pub fn new_shared(
        text: &'static str,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(text, remote_ip, remote_port))
    }
}

impl Application for SendMessage {
    const ID: ProtocolId = ProtocolId::from_string("Send Message");

    fn start(
        self: Arc<Self>,
        context: Context,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, self.ip);
        LocalPort::set(&mut participants, 0);
        RemotePort::set(&mut participants, self.port);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants, context.clone())?;
        session.send(Message::new(self.text), context)?;
        Ok(())
    }

    fn recv(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
