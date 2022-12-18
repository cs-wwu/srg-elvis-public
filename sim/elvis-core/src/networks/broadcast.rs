use super::Mtu;
use crate::{
    control::{Key, Primitive},
    network::{SharedTap, Tap, TapEnvironment},
    protocol::Context,
    session::QueryError,
    Message, Network,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

pub struct Broadcast {
    // TODO(hardint): Add a way to access the MTU by other protocols
    // TODO(hardint): Only allow messages up to `mtu` in size
    /// The maximum transmission unit of the network
    #[allow(dead_code)]
    mtu: Mtu,
    to_network: (mpsc::Sender<Message>, mpsc::Receiver<Message>),
    from_network: (broadcast::Sender<Message>, broadcast::Receiver<Message>),
}

impl Broadcast {
    pub fn new(mtu: Mtu) -> Self {
        Self {
            mtu,
            to_network: mpsc::channel::<Message>(16),
            from_network: broadcast::channel::<Message>(16),
        }
    }
}

impl Network for Broadcast {
    fn start(mut self: Box<Self>) {
        tokio::spawn(async move {
            while let Some(message) = self.to_network.1.recv().await {
                match self.from_network.0.send(message) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Failed to send on broadcast network: {}", e);
                    }
                }
            }
        });
    }

    fn tap(&mut self) -> SharedTap {
        Arc::new(BroadcastTap::new(
            self.to_network.0.clone(),
            self.from_network.0.subscribe(),
        ))
    }
}

pub struct BroadcastTap {
    send: mpsc::Sender<Message>,
    receive: Arc<Mutex<Option<broadcast::Receiver<Message>>>>,
}

impl BroadcastTap {
    pub fn new(send: mpsc::Sender<Message>, receive: broadcast::Receiver<Message>) -> Self {
        Self {
            send,
            receive: Arc::new(Mutex::new(Some(receive))),
        }
    }
}

impl Tap for BroadcastTap {
    fn start(self: Arc<Self>, environment: TapEnvironment) {
        let mut receive = self.receive.lock().unwrap().take().unwrap();
        tokio::spawn(async move {
            while let Ok(message) = receive.recv().await {
                let environment = environment.clone();
                let context = Context::new(environment.protocols);
                let _ = environment.session.receive(message, context);
            }
        });
    }

    fn send(self: Arc<Self>, message: Message) {
        tokio::spawn(async move {
            match self.send.send(message).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to send on broadcast network: {}", e);
                }
            }
        });
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        todo!()
    }
}
