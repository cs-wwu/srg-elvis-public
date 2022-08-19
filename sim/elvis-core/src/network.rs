use super::machine::MachineId;
use crate::{protocols::tap::NetworkId, Message};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Delivery {
    pub message: Message,
    pub network: NetworkId,
    pub sender: MachineId,
}

pub trait Network {
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery>;
}

#[derive(Clone)]
pub struct Attachment {
    pub machine: MachineId,
    pub sender: Sender<Delivery>,
}
