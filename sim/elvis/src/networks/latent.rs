use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, Sender};

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

impl Network for Latent {
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery> {
        let (sender, mut receiver) = mpsc::channel::<Delivery>(16);
        tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                tokio::time::sleep(self.latency).await;
                for attachment in attachments
                    .iter()
                    .filter(|attachment| attachment.machine != delivery.sender)
                {
                    attachment.sender.send(delivery.clone()).await.unwrap();
                }
            }
        });
        sender
    }
}
