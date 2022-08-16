use super::machine::MachineId;
use crate::protocols::tap::Delivery;
use async_trait::async_trait;
use std::{error::Error, sync::Arc};
use tokio::sync::mpsc::Sender;

pub type SharedNetwork = Arc<dyn Network + Send + Sync + 'static>;

#[async_trait]
pub trait Network {
    async fn send(
        self: Arc<Self>,
        delivery: Delivery,
        attachments: &[Attachment],
    ) -> Result<(), Box<dyn Error>>;
}

#[derive(Clone)]
pub struct Attachment {
    pub machine: MachineId,
    pub sender: Sender<Delivery>,
}
