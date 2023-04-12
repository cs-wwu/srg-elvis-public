use dashmap::mapref::entry::Entry;

use super::socket::Socket;
use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError, SharedSession},
    FxDashMap, Id, Message, Session,
};
use std::sync::{Arc, RwLock};

pub(super) struct SocketSession {
    pub upstream: RwLock<Option<Id>>,
    pub downstream: SharedSession,
    pub sockets: Arc<FxDashMap<Id, Arc<Socket>>>,
}

impl SocketSession {
    pub fn receive(self: Arc<Self>, message: Message) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => match self.sockets.entry(sock) {
                Entry::Occupied(entry) => entry.get().receive(message),
                Entry::Vacant(_) => Err(DemuxError::MissingSession),
            },
            None => Ok(()),
        }
    }
}

impl Session for SocketSession {
    fn send(&self, message: Message, context: Context) -> Result<(), SendError> {
        self.downstream.send(message, context)
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.query(key)
    }
}
