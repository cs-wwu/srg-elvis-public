use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, Sender};

pub struct Latent {
    latency: Duration,
}

impl Latent {
    pub fn new(latency: Duration) -> Self {
        Self { latency }
    }
}

impl Network for Latent {
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery> {
        let (sender, mut receiver) = mpsc::channel::<Delivery>(16);
        tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                tokio::time::sleep(self.latency).await;
                for attachment in attachments.iter() {
                    attachment.sender.send(delivery.clone()).await.unwrap();
                }
            }
        });
        sender
    }
}
