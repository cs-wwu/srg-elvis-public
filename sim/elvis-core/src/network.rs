//! Contains the [`Network`] trait and supporting types.

use tokio::sync::Barrier;

use crate::{
    control::{Key, Primitive},
    machine::ProtocolMap,
    protocol::Context,
    session::{QueryError, SendError, SharedSession},
    Control, Message,
};
use std::sync::Arc;

pub type SharedTap = Arc<dyn Tap + Send + Sync + 'static>;
pub type TapIndex = u32;

pub trait Tap {
    /// Spawns a task for the Network to run in and returns half a channel on
    /// which to send messages to the network.
    fn start(self: Arc<Self>, environment: TapEnvironment, barrier: Arc<Barrier>);

    /// Takes the message, appends headers, and forwards it to the next session
    /// in the chain for further processing.
    fn send(self: Arc<Self>, message: Message, control: Control) -> Result<(), SendError>;

    /// Gets a piece of information from some session in the protocol stack.
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError>;
}

#[derive(Clone)]
pub struct TapEnvironment {
    pub protocols: ProtocolMap,
    pub session: SharedSession,
}

impl TapEnvironment {
    pub fn new(protocols: ProtocolMap, session: SharedSession) -> Self {
        Self { protocols, session }
    }

    pub fn context(&self) -> Context {
        Context::new(self.protocols.clone())
    }
}
