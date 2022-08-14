use super::machine::MachineId;
use crate::protocols::tap::Delivery;
use std::{error::Error, sync::Arc};
use tokio::sync::mpsc::Sender;

pub trait Network {
    fn send(
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
