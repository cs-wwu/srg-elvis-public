use crate::{network::Attachment, protocols::tap::Delivery, Network};
use async_trait::async_trait;
use std::{error::Error, sync::Arc};

use super::Mtu;

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

#[async_trait]
impl Network for Reliable {
    async fn send(
        self: Arc<Self>,
        delivery: Delivery,
        attachments: &[Attachment],
    ) -> Result<(), Box<dyn Error>> {
        for attachment in attachments {
            attachment.sender.send(delivery.clone()).await.unwrap();
        }
        Ok(())
    }
}
