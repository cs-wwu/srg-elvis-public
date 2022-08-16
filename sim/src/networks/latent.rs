use crate::{
    core::{network::Attachment, Network},
    protocols::tap::Delivery,
};
use async_trait::async_trait;
use std::{error::Error, sync::Arc};

pub struct Latent {}

#[async_trait]
impl Network for Latent {
    async fn send(
        self: Arc<Self>,
        delivery: Delivery,
        attachments: &[Attachment],
    ) -> Result<(), Box<dyn Error>> {
        for attachment in attachments
            .iter()
            .filter(|attachment| attachment.machine != delivery.sender)
        {
            attachment.sender.send(delivery.clone()).await.unwrap();
        }
        Ok(())
    }
}
