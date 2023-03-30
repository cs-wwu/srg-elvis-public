use std::sync::{Mutex, Arc};

use tokio::sync::{RwLock, mpsc::{UnboundedSender, self}, Barrier, broadcast};

use crate::{protocol::{Context, StartError, OpenError, ListenError, DemuxError, QueryError}, Message, Control, Protocol, ProtocolMap, Id, session::SharedSession, control::{Key, Primitive}};

use super::Pci;

pub struct SubscribablePci {
    /// The Pci object backing this
    inner: Pci,
    /// send on this when a message is sent upstream
    upstream_receiver: mpsc::UnboundedReceiver<(Message, Context)>,
    /// send on this when a message is sent downstream
    downstream_receiver: mpsc::UnboundedReceiver<(Message, Context)>,
}

impl SubscribablePci {
    pub const ID: Id = Pci::ID;
}

impl Protocol for SubscribablePci {
    fn id(self: Arc<Self>) -> crate::Id {
        Pci::ID
    }

    fn start(
        self: Arc<Self>,
        shutdown: mpsc::Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Start thread to forward messages from SubscribablePci to subscribers

        todo!()
    }

    fn open(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        todo!()
    }

    fn listen(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        todo!()
    }

    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        todo!()
    }
}