use async_trait::async_trait;
use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use std::{error::Error, sync::Arc, time::Duration};

pub struct Latent {
    latency: Duration,
}

impl Latent {
    pub fn new(latency: Duration) -> Self {
        Self { latency }
    }
}

#[async_trait]
impl Network for Latent {
    async fn send(
        self: Arc<Self>,
        delivery: Delivery,
        attachments: &[Attachment],
    ) -> Result<(), Box<dyn Error>> {
        // This does not block other sends on this network, just this one
        tokio::time::sleep(self.latency).await;
        for attachment in attachments {
            attachment.sender.send(delivery.clone()).await.unwrap();
        }
        Ok(())
    }
}
