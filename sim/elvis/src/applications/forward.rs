use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
    },
    session::SharedSession,
    Control, Id, Participants, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;
/// An application that forwards messages to `local_ip` to `remote_ip`.
pub struct Forward {
    /// The session on which we send any messages we receive
    outgoing: RwLock<Option<SharedSession>>,
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
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for Forward {
    const ID: Id = Id::from_string("Forward");

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut participants = Participants::new();
        participants.local.port = Some(self.local_port);
        participants.local.address = Some(self.local_ip);
        participants.remote.port = Some(self.remote_port);
        participants.remote.address = Some(self.remote_ip);

        let udp = protocols.protocol(Udp::ID).expect("No such protocol");
        *self.outgoing.write().unwrap() = Some(udp.open(
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
        &self,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        self.outgoing
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(message, control, protocols)?;
        Ok(())
    }
}
