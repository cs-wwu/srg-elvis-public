use dashmap::mapref::entry::Entry;
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::ipv4_parsing::Ipv4Header,
        Endpoint, Endpoints, Tcp, Udp, udp::UdpHeader,
    },
    Control, Protocol, Session, Shutdown, Transport, FxDashMap,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

pub struct BasicServer {
    /// The endpoint to send to
    endpoint: Endpoint,
    /// Whether to use UDP or TCP
    transport: Transport,
    /// Whether to output text or not
    output: bool,
    /// Whether a request has been received
    received: FxDashMap<Endpoints, Arc<dyn Session>>,
    /// The number of clients
    num_clients: u8,
    /// The number of served clients
    served_clients: RwLock<u8>,
    /// The channel to send a shutdown on
    shutdown: RwLock<Option<Shutdown>>,
}

impl BasicServer {
    pub fn new(
        endpoint: Endpoint,
        transport: Transport,
        output: bool,
        num_clients: u8
    ) -> Self {
        Self {
            endpoint,
            transport,
            output,
            received: Default::default(),
            num_clients,
            served_clients: RwLock::new(0),
            shutdown: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for BasicServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        match self.transport {
            Transport::Tcp => {
                protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols)
                    .unwrap();
            }
            Transport::Udp => {
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols)
                    .unwrap();
            }
        }
        *self.shutdown.write().unwrap() = Some(shutdown);
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let identifier = match control.get::<Endpoints>() {
            Some(endpoints) => *endpoints,
            None => Endpoints::new_from_headers(
                control.get::<UdpHeader>(),
                control.get::<Ipv4Header>(),
            )?,
        };
        match self.received.entry(identifier) {
            Entry::Occupied(_) => {
                if self.output {
                    println!(
                        "SERVER: Acknowledgement Received: {:?}",
                        String::from_utf8(message.to_vec()).unwrap()
                    )
                }
                *self.served_clients.write().unwrap() += 1;
                if *self.served_clients.read().unwrap() >= self.num_clients {
                    if self.output {
                        println!("SERVER: Shutting down");
                    }
                    match *self.shutdown.write().unwrap() {
                        Some(ref shutdown) => shutdown.shut_down(),
                        None => { },
                    }
                }
            },
            Entry::Vacant(entry) => {
                if self.output {
                    println!(
                        "SERVER: Request Received: {:?}",
                        String::from_utf8(message.to_vec()).unwrap()
                    )
                }
                let rsp = format!("Major Tom to Ground Control");
                if self.output {
                    println!(
                        "SERVER: Sending Response: {:?}",
                        rsp
                    )
                }
                entry.insert(caller.clone());
                caller.send(Message::new(rsp), protocols).unwrap();
            },
        }
        Ok(())
    }
}
