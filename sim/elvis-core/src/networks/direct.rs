use futures::{stream::FuturesUnordered, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    control::{Key, Primitive},
    machine::TapSlot,
    network::{OpaqueNetwork, SharedTap, Tap, TapEnvironment},
    session::QueryError,
    Message, Network,
};

pub struct Direct {
    to_network: (mpsc::Sender<Delivery>, mpsc::Receiver<Delivery>),
    senders: Vec<mpsc::Sender<Delivery>>,
    receivers: Vec<mpsc::Receiver<Delivery>>,
}

impl Direct {
    pub fn new() -> Self {
        Self {
            to_network: mpsc::channel(16),
            senders: vec![],
            receivers: vec![],
        }
    }

    pub fn new_opaque() -> OpaqueNetwork {
        Box::new(Self::new())
    }
}

impl Network for Direct {
    fn start(self: Box<Self>) {
        tokio::spawn(async move {
            let receivers: FuturesUnordered<_> = self
                .receivers
                .into_iter()
                .map(|mut receiver| receiver.recv())
                .collect();
            while let Some(Some(delivery)) = receivers.next().await {
                match self.senders[delivery.recipient as usize]
                    .send(delivery)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Failed to send on broadcast network: {}", e);
                    }
                }
            }
        });
    }

    fn tap(&mut self) -> SharedTap {
        let (to_network_sender, to_network_receiver) = mpsc::channel(16);
        let (to_tap_sender, to_tap_receiver) = mpsc::channel(16);
        self.senders.push(to_tap_sender);
        self.receivers.push(to_network_receiver);
        Arc::new(DirectTap::new(
            self.senders.len() as u32,
            to_network_sender,
            to_tap_receiver,
        ))
    }
}

pub struct DirectTap {
    mac: u32,
    send: mpsc::Sender<Delivery>,
    receive: mpsc::Receiver<Delivery>,
}

impl DirectTap {
    pub fn new(mac: u32, send: mpsc::Sender<Delivery>, receive: mpsc::Receiver<Delivery>) -> Self {
        Self { mac, send, receive }
    }
}

impl Tap for DirectTap {
    fn start(self: Arc<Self>, environment: TapEnvironment) {
        todo!()
    }

    fn send(self: Arc<Self>, message: Message) {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct Delivery {
    message: Message,
    recipient: TapSlot,
}
