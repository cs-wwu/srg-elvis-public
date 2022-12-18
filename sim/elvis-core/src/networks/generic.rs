use crate::{
    control::{Key, Primitive},
    network::{OpaqueNetwork, SharedTap, Tap, TapEnvironment},
    protocol::{Context, ProtocolId},
    session::{QueryError, SendError},
    Control, Message, Network,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc,
};

use super::get_destination_mac;

type DirectConnections = Arc<RwLock<Vec<mpsc::Sender<Message>>>>;

pub struct Generic {
    connections: DirectConnections,
    broadcast: broadcast::Sender<Message>,
}

impl Generic {
    pub const ID: ProtocolId = ProtocolId::from_string("Direct network");

    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(vec![])),
            broadcast: broadcast::channel::<Message>(16).0,
        }
    }

    pub fn new_opaque() -> OpaqueNetwork {
        Box::new(Self::new())
    }
}

impl Network for Generic {
    fn start(self: Box<Self>) {}

    fn tap(&mut self) -> SharedTap {
        let (send, receive) = mpsc::channel(16);
        self.connections.write().unwrap().push(send);
        Arc::new(DirectTap::new(
            self.connections.clone(),
            receive,
            self.broadcast.clone(),
        ))
    }
}

pub struct DirectTap {
    connections: DirectConnections,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
    broadcast: broadcast::Sender<Message>,
}

impl DirectTap {
    pub fn new(
        connections: DirectConnections,
        receiver: mpsc::Receiver<Message>,
        broadcast: broadcast::Sender<Message>,
    ) -> Self {
        Self {
            connections,
            direct_receiver: Arc::new(RwLock::new(Some(receiver))),
            broadcast,
        }
    }
}

impl Tap for DirectTap {
    fn start(self: Arc<Self>, environment: TapEnvironment) {
        let mut direct_receiver = self.direct_receiver.write().unwrap().take().unwrap();
        let mut broadcast_receiver = self.broadcast.subscribe();
        tokio::spawn(async move {
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

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        todo!()
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
