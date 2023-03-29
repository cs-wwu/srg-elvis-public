use elvis_core::{
    message::Message,
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

/// An application that sends a Time To Live (TTL) to
/// another machine from the first machine.
/// The second machine will then send the TTL back minus 1.
/// Once the TTL reaches 0 the program ends.
pub struct PingPong {
    /// The channel we send on to shut down the simulation
    shutdown: RwLock<Option<Shutdown>>,
    /// The session we send messages on
    session: RwLock<Option<SharedSession>>,
    is_initiator: bool,
    /// The address we listen for a message on
    local_ip_address: Ipv4Address,
    remote_ip_address: Ipv4Address,
    /// The port we listen for a message on
    local_port: u16,
    remote_port: u16,
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
            shutdown: Default::default(),
            session: Default::default(),
            local_ip_address,
            remote_ip_address,
            local_port,
            remote_port,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for PingPong {
    const ID: Id = Id::from_string("PingPong");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);

        let mut participants = Control::new();
        Ipv4::set_local_address(self.local_ip_address, &mut participants);
        Ipv4::set_remote_address(self.remote_ip_address, &mut participants);
        Udp::set_local_port(self.local_port, &mut participants);
        Udp::set_remote_port(self.remote_port, &mut participants);
        let protocol = protocols.protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants, protocols.clone())?;
        *self.session.write().unwrap() = Some(session.clone());

        let context = Context::new(protocols);
        let is_initiator = self.is_initiator;
        tokio::spawn(async move {
            initialized.wait().await;
            if is_initiator {
                session
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
        } else {
            tracing::info!("Ping {}", ttl);
        }

        let ttl = ttl - 1;

        if ttl == 0 {
            tracing::info!("TTL has reach 0, PingPong has successfully completed");
            if let Some(shutdown) = self.shutdown.write().unwrap().take() {
                shutdown.shut_down();
            }
        } else {
            self.session
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .clone()
                .send(Message::new(vec![ttl]), context)?;
        }
        Ok(())
    }
}
