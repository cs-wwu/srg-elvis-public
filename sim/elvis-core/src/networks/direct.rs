use crate::{
    control::{ControlError, Key, Primitive},
    network::{OpaqueNetwork, SharedTap, Tap, TapEnvironment},
    protocol::ProtocolId,
    session::{QueryError, SendError},
    Control, Message, Network,
};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{self, Receiver, Sender};

type DirectConnections = Arc<RwLock<Vec<Sender<Message>>>>;

pub struct Direct {
    connections: DirectConnections,
}

impl Direct {
    pub const ID: ProtocolId = ProtocolId::from_string("Direct network");

    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn new_opaque() -> OpaqueNetwork {
        Box::new(Self::new())
    }

    pub fn set_destination_mac(mac: u64, control: &mut Control) {
        control.insert((Self::ID, 0), mac);
    }

    pub fn get_destination_mac(control: &Control) -> Result<u64, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u64()?)
    }
}

impl Network for Direct {
    fn start(self: Box<Self>) {}

    fn tap(&mut self) -> SharedTap {
        let (send, receive) = mpsc::channel(16);
        self.connections.write().unwrap().push(send);
        Arc::new(DirectTap::new(self.connections.clone(), receive))
    }
}

pub struct DirectTap {
    connections: DirectConnections,
    receiver: Arc<RwLock<Option<Receiver<Message>>>>,
}

impl DirectTap {
    pub fn new(connections: DirectConnections, receiver: Receiver<Message>) -> Self {
        Self {
            connections,
            receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }
}

impl Tap for DirectTap {
    fn start(self: Arc<Self>, environment: TapEnvironment) {
        let mut receiver = self.receiver.write().unwrap().take().unwrap();
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
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
        });
    }

    fn send(self: Arc<Self>, message: Message, control: Control) -> Result<(), SendError> {
        let destination = Direct::get_destination_mac(&control).or_else(|_| {
            tracing::error!("Missing destination mac on context");
            Err(SendError::MissingContext)
        })? as usize;
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

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        todo!()
    }
}
