//! Contains the [`Network`] trait and supporting types.

use super::machine::MachineId;
use crate::{protocols::tap::NetworkId, Message};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// A message in transit over a [`Network`].
#[derive(Debug, Clone)]
pub struct Delivery {
    /// The message to deliver.
    pub message: Message,
    /// The ID of the network the message is being sent over.
    pub network: NetworkId,
    /// The machine that sent the message.
    pub sender: MachineId,
}

/// Models a network that connects machine and delivers
/// messages between them.
pub trait Network {
    /// Spawns a task for the Network to run in and returns half a channel on
    /// which to send messages to the network.
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery>;
}

/// Information about a particular machine connected to a
/// [`Network`].
#[derive(Clone)]
pub struct Attachment {
    /// The ID of the machine attached to the [`Network`].
    pub machine: MachineId,
    /// The channel to send messages on to deliver them to the `machine`.
    pub sender: Sender<Delivery>,
}
