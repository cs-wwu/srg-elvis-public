use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Udp,
    },
    session::SharedSession,
    Control, Participants, Protocol, Shutdown, Transport,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
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
            shutdown: Default::default(),
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
    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
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
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        *self.shutdown.write().unwrap() = Some(shutdown);

        let mut participants = Participants::new();
        participants.local.address = Some(self.local_ip_address);
        participants.local.port = Some(self.local_port);
        participants.remote.address = Some(self.remote_ip_address);
        participants.remote.port = Some(self.remote_port);

        let protocol = protocols.protocol::<Udp>().expect("No such protocol");
        let session = protocol.open(TypeId::of::<Self>(), participants, protocols.clone())?;
        *self.session.write().unwrap() = Some(session.clone());

        let is_initiator = self.is_initiator;
        tokio::spawn(async move {
            initialized.wait().await;
            if is_initiator {
                session
                    //Send the first "Ping" message with TTL of 255
                    .send(Message::new(vec![255]), Control::new(), protocols)
                    .unwrap();
            }
        });
        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
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
            self.session.read().unwrap().as_ref().unwrap().send(
                Message::new(vec![ttl]),
                control,
                protocols,
            )?;
        }
        Ok(())
    }
}
