use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocol::{DemuxError, StartError},
    protocols::{Endpoints, Udp},
    Control, Protocol, Session, Shutdown, Transport,
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
    session: RwLock<Option<Arc<dyn Session>>>,
    is_initiator: bool,
    endpoints: Endpoints,
    /// The machine that will receive the message
    remote_mac: Option<Mac>,
    /// The protocol to use in delivering the message
    transport: Transport,
}

impl PingPong {
    /// Creates a new capture.
    pub fn new(is_initiator: bool, endpoints: Endpoints) -> Self {
        Self {
            is_initiator,
            shutdown: Default::default(),
            session: Default::default(),
            endpoints,
            remote_mac: None,
            transport: Transport::Udp,
        }
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

#[async_trait::async_trait]
impl Protocol for PingPong {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        let protocol = protocols.protocol::<Udp>().expect("No such protocol");
        let session = protocol
            .open_and_listen(self.id(), self.endpoints, protocols.clone())
            .await
            .unwrap();
        *self.session.write().unwrap() = Some(session.clone());

        let is_initiator = self.is_initiator;
        initialized.wait().await;
        if is_initiator {
            session
                //Send the first "Ping" message with TTL of 255
                .send(Message::new(vec![255]), protocols)
                .unwrap();
        }
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let ttl = message.iter().next().expect("The message contained no TTL");
        print!("ping pong got message");
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
                .send(Message::new(vec![ttl]), protocols)?;
        }
        Ok(())
    }
}
