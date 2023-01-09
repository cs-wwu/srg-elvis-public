use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::Ipv4Address,
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4,
    },
    session::SharedSession,
    Control, ProtocolMap,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::Sender, Barrier};
/// An application that forwards messages to `local_ip` to `remote_ip`.
#[derive(Clone)]
pub struct Forward {
    /// The session on which we send any messages we receive
    outgoing: Arc<Mutex<Option<SharedSession>>>,
    /// The IP address for incoming messages
    local_ip: Ipv4Address,
    /// The IP address for outgoing messages
    remote_ip: Ipv4Address,
    /// The port number for incoming messages
    local_port: u16,
    /// The port number for outgoing messages
    remote_port: u16,
}

impl Forward {
    /// Creates a new forwarding application.
    pub fn new(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            outgoing: Default::default(),
            local_ip,
            remote_ip,
            local_port,
            remote_port,
        }
    }

    /// Creates a new forwarding application behind a shared handle.
    pub fn new_shared(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(local_ip, remote_ip, local_port, remote_port))
    }
}

impl Application for Forward {
    const ID: ProtocolId = ProtocolId::from_string("Forward");

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(self.local_ip, &mut participants);
        Ipv4::set_remote_address(self.remote_ip, &mut participants);
        Udp::set_local_port(self.local_port, &mut participants);
        Udp::set_remote_port(self.remote_port, &mut participants);

        let udp = protocols.protocol(Udp::ID).expect("No such protocol");
        *self.outgoing.lock().unwrap() = Some(udp.clone().open(
            Self::ID,
            // TODO(hardint): Can these clones be cheaper?
            participants.clone(),
            protocols.clone(),
        )?);
        udp.listen(Self::ID, participants, protocols)?;
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        message: Message,
        context: Context,
    ) -> Result<(), ApplicationError> {
        // TODO(hardint): Use ? again
        self.outgoing
            .clone()
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
            .send(message, context)?;
        Ok(())
    }
}
