use crate::{
    control::{Key, Primitive},
    id::Id,
    network::{SharedTap, Tap, TapEnvironment},
    protocol::Context,
    session::{QueryError, SendError},
    Control, Message,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc, Barrier,
};

use super::{get_destination_mac, Mtu};

type DirectConnections = Arc<RwLock<Vec<mpsc::Sender<Message>>>>;

pub struct Generic {
    mtu: Mtu,
    connections: DirectConnections,
    broadcast: broadcast::Sender<Message>,
}

impl Generic {
    pub const ID: Id = Id::from_string("Direct network");
    pub const MTU_QUERY_KEY: Key = (Self::ID, 0);

    pub fn new(mtu: Mtu) -> Self {
        Self {
            mtu,
            connections: Arc::new(RwLock::new(vec![])),
            broadcast: broadcast::channel::<Message>(16).0,
        }
    }

    pub fn tap(&mut self) -> SharedTap {
        let (send, receive) = mpsc::channel(16);
        self.connections.write().unwrap().push(send);
        Arc::new(GenericTap::new(
            self.mtu,
            self.connections.clone(),
            receive,
            self.broadcast.clone(),
        ))
    }
}

pub struct GenericTap {
    mtu: Mtu,
    connections: DirectConnections,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
    broadcast: broadcast::Sender<Message>,
}

impl GenericTap {
    pub fn new(
        mtu: Mtu,
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
}

impl Tap for GenericTap {
    fn start(self: Arc<Self>, environment: TapEnvironment, barrier: Arc<Barrier>) {
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

    fn send(self: Arc<Self>, message: Message, control: Control) -> Result<(), SendError> {
        if message.len() > self.mtu as usize {
            Err(SendError::Mtu(self.mtu))?
        }

        match get_destination_mac(&control) {
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

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Generic::MTU_QUERY_KEY => Ok(self.mtu.into()),
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
