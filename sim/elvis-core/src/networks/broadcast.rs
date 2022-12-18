use super::Mtu;
use crate::{
    control::{Key, Primitive},
    network::{OpaqueNetwork, SharedTap, Tap, TapEnvironment},
    protocol::Context,
    session::{QueryError, SendError},
    Control, Message, Network,
};
use std::sync::Arc;
use tokio::sync::broadcast::{self, Sender};

pub struct Broadcast {
    // TODO(hardint): Add a way to access the MTU by other protocols
    // TODO(hardint): Only allow messages up to `mtu` in size
    /// The maximum transmission unit of the network
    #[allow(dead_code)]
    mtu: Mtu,
    send: Sender<Message>,
}

impl Broadcast {
    pub fn new(mtu: Mtu) -> Self {
        Self {
            mtu,
            send: broadcast::channel::<Message>(16).0,
        }
    }

    pub fn new_opaque(mtu: Mtu) -> OpaqueNetwork {
        Box::new(Self::new(mtu))
    }
}

impl Network for Broadcast {
    fn start(self: Box<Self>) {}

    fn tap(&mut self) -> SharedTap {
        Arc::new(BroadcastTap::new(self.send.clone()))
    }
}

pub struct BroadcastTap {
    send: Sender<Message>,
}

impl BroadcastTap {
    pub fn new(send: Sender<Message>) -> Self {
        Self { send }
    }
}

impl Tap for BroadcastTap {
    fn start(self: Arc<Self>, environment: TapEnvironment) {
        let mut receive = self.send.subscribe();
        tokio::spawn(async move {
            while let Ok(message) = receive.recv().await {
                let environment = environment.clone();
                let context = Context::new(environment.protocols);
                match environment.session.receive(message, context) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Failed to receive on a broadcast network: {}", e);
                    }
                }
            }
        });
    }

    fn send(self: Arc<Self>, message: Message, _control: Control) -> Result<(), SendError> {
        match self.send.send(message) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to send on broadcast network: {}", e);
                Err(SendError::Other)
            }
        }
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        todo!()
    }
}
