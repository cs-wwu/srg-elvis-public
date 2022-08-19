use async_trait::async_trait;
use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use std::{error::Error, sync::Arc, time::Duration};

/// A network that takes some amount of time to transfer messages through.
///
/// Note that this is different from low bandwidth. Messages can be sent at an
/// arbitrarily high rate, but there is a delay before they arrive.
pub struct Latent {
    /// The amount of time between when a message is sent and when it is
    /// delivered.
    latency: Duration,
}

impl Latent {
    /// Creates a new instance of the network with the given latency.
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
            // We don't want to unwrap the send. It might be that the channel
            // was closed as part of a graceful shutdown, which we should not
            // crash in response to.
            match attachment.sender.send(delivery.clone()).await {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e),
            }
        }
        Ok(())
    }
}
