use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    protocol::Context,
    session::{QueryError, SendError, SharedSession},
    Control, Message,
};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc, Barrier,
};

pub type TapIndex = u32;
type DirectConnections = Arc<RwLock<Vec<mpsc::Sender<Message>>>>;

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;
pub type Mac = u64;

pub struct Network {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    connections: DirectConnections,
    broadcast: broadcast::Sender<Message>,
}

impl Network {
    pub const ID: Id = Id::from_string("Network");
    pub const MTU_QUERY_KEY: Key = (Self::ID, 0);

    pub fn new() -> Self {
        Self {
            mtu: None,
            latency: None,
            connections: Arc::new(RwLock::new(vec![])),
            broadcast: broadcast::channel::<Message>(16).0,
        }
    }

    pub fn mtu(mut self, mtu: Mtu) -> Self {
        self.mtu = Some(mtu);
        self
    }

    pub fn latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    pub fn tap(&mut self) -> Tap {
        let (send, receive) = mpsc::channel(16);
        self.connections.write().unwrap().push(send);
        Tap::new(
            self.mtu,
            self.connections.clone(),
            receive,
            self.broadcast.clone(),
        )
    }

    pub fn set_destination_mac(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 0), mac);
    }

    pub fn get_destination_mac(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u64()?)
    }
}

pub struct Tap {
    mtu: Option<Mtu>,
    connections: DirectConnections,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
    broadcast: broadcast::Sender<Message>,
}

impl Tap {
    pub fn new(
        mtu: Option<Mtu>,
        connections: DirectConnections,
        receiver: mpsc::Receiver<Message>,
        broadcast: broadcast::Sender<Message>,
    ) -> Self {
        Self {
            mtu,
            connections,
            direct_receiver: Arc::new(RwLock::new(Some(receiver))),
            broadcast,
        }
    }

    pub(crate) fn start(&self, environment: TapEnvironment, barrier: Arc<Barrier>) {
        let mut direct_receiver = self.direct_receiver.write().unwrap().take().unwrap();
        let mut broadcast_receiver = self.broadcast.subscribe();
        tokio::spawn(async move {
            barrier.wait().await;
            loop {
                tokio::select! {
                    message = direct_receiver.recv() => {
                        receive_direct(message, environment.clone());
                    }
                    message = broadcast_receiver.recv() => {
                        receive_broadcast(message, environment.clone());
                    }
                }
            }
        });
    }

    pub(crate) fn send(&self, message: Message, control: Control) -> Result<(), SendError> {
        if let Some(mtu) = self.mtu {
            if message.len() > mtu as usize {
                Err(SendError::Mtu(mtu))?
            }
        }

        match Network::get_destination_mac(&control) {
            Ok(destination) => {
                let destination = destination as usize;
                let channel = self
                    .connections
                    .read()
                    .unwrap()
                    .get(destination)
                    .ok_or_else(|| {
                        tracing::error!("The destination mac is out of bounds: {}", destination);
                        SendError::MissingContext
                    })?
                    .clone();
                tokio::spawn(async move {
                    match channel.clone().send(message).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to send on direct network: {}", e);
                        }
                    }
                });
                Ok(())
            }

            Err(_) => match self.broadcast.send(message) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::error!("Failed to send on broadcast network: {}", e);
                    Err(SendError::Other)
                }
            },
        }
    }

    pub(crate) fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Network::MTU_QUERY_KEY => Ok(self.mtu.unwrap_or(0).into()),
            _ => Err(QueryError::MissingKey),
        }
    }
}

fn receive_direct(message: Option<Message>, environment: TapEnvironment) {
    if let Some(message) = message {
        match environment
            .session
            .clone()
            .receive(message, environment.context())
        {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to receive on direct network: {}", e);
            }
        }
    }
}

fn receive_broadcast(message: Result<Message, RecvError>, environment: TapEnvironment) {
    match message {
        Ok(message) => {
            let context = Context::new(environment.protocols);
            match environment.session.receive(message, context) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to receive on a broadcast network: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Broadcast receive error: {}", e);
        }
    }
}

#[derive(Clone)]
pub struct TapEnvironment {
    pub protocols: ProtocolMap,
    pub session: SharedSession,
}

impl TapEnvironment {
    pub fn new(protocols: ProtocolMap, session: SharedSession) -> Self {
        Self { protocols, session }
    }

    pub fn context(&self) -> Context {
        Context::new(self.protocols.clone())
    }
}
