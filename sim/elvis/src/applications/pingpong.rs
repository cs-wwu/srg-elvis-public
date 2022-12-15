use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort},
        user_process::{Application, UserProcess},
        Udp,
    },
    session::SharedSession,
    Control,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::Sender, Barrier};

/// An application that sends a Time To Live (TTL) to
/// another machine from the first machine.
/// The second machine will then send the TTL back minus 1.
/// Once the TTL reaches 0 the program ends.
#[derive(Clone)]
pub struct PingPong {
    /// The channel we send on to shut down the simulation
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    /// The session we send messages on
    session: Arc<Mutex<Option<SharedSession>>>,
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
    pub fn new_shared(
        is_initiator: bool,
        local_ip_address: Ipv4Address,
        remote_ip_address: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            is_initiator,
            local_ip_address,
            remote_ip_address,
            local_port,
            remote_port,
        ))
    }
}

impl Application for PingPong {
    const ID: ProtocolId = ProtocolId::from_string("PingPong");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), ()> {
        *self.shutdown.lock().unwrap() = Some(shutdown);

        let mut participants = Control::new();
        LocalAddress::set(&mut participants, self.local_ip_address);
        RemoteAddress::set(&mut participants, self.remote_ip_address);
        LocalPort::set(&mut participants, self.local_port);
        RemotePort::set(&mut participants, self.remote_port);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let session = protocol.open(Self::ID, participants, context.clone())?;
        *self.session.lock().unwrap() = Some(session);

        tokio::spawn(async move {
            initialized.wait().await;
            if self.is_initiator {
                self.session
                    .clone()
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .clone()
                    //Send the first "Ping" message with TTL of 255
                    .send(Message::new(vec![255]), context)
                    .unwrap();
            }
        });
        Ok(())
    }

    fn recv(self: Arc<Self>, message: Message, context: Context) -> Result<(), ()> {
        let ttl = match message.iter().next() {
            Some(ttl) => ttl,
            None => {
                tracing::error!("The message contained no TTL");
                Err(())?
            }
        };

        if ttl % 2 == 0 {
            tracing::info!("Pong {}", ttl);
        } else {
            tracing::info!("Ping {}", ttl);
        }

        let ttl = ttl - 1;

        if ttl == 0 {
            tracing::info!("TTL has reach 0, PingPong has successfully completed");
            if let Some(shutdown) = self.shutdown.lock().unwrap().take() {
                tokio::spawn(async move {
                    shutdown.send(()).await.unwrap();
                });
            }
        } else {
            self.session
                .clone()
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .clone()
                .send(Message::new(vec![ttl]), context)?;
        }
        Ok(())
    }
}
