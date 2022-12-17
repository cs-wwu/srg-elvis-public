use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::Ipv4Address,
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4,
    },
    Control,
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
        initialized: Arc<Barrier>,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(Ipv4Address::LOCALHOST, &mut participants);
        Ipv4::set_remote_address(self.ip, &mut participants);
        Udp::set_local_port(0, &mut participants);
        Udp::set_remote_port(self.port, &mut participants);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants, context.clone())?;
        tokio::spawn(async move {
            initialized.wait().await;
            session
                .send(Message::new(self.text), context)
                .expect("SendMessage failed to send");
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
