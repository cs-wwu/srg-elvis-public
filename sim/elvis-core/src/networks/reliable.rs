use super::Mtu;
use crate::{
    network::{Attachment, Delivery},
    Network,
};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};

pub struct Reliable {
    // TODO(hardint): Add a way to access the MTU by other protocols
    // TODO(hardint): Only allow messages up to `mtu` in size
    /// The maximum transmission unit of the network
    #[allow(dead_code)]
    mtu: Mtu,
}

impl Reliable {
    pub fn new(mtu: Mtu) -> Self {
        Self { mtu }
    }
}

impl Network for Reliable {
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery> {
        let (sender, mut receiver) = mpsc::channel::<Delivery>(16);
        tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                for attachment in attachments.iter() {
                    attachment.sender.send(delivery.clone()).await.unwrap();
                }
            }
        });
        sender
    }
}
