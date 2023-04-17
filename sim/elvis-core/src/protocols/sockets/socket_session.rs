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
    pub stored_msg: RwLock<Option<Message>>,
    pub sockets: Arc<FxDashMap<Id, Arc<Socket>>>,
}

impl SocketSession {
    pub fn receive(&self, message: Message) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => match self.sockets.entry(sock) {
                Entry::Occupied(entry) => entry.get().receive(message),
                Entry::Vacant(_) => Err(DemuxError::MissingSession),
            },
            None => Ok(()),
        }
    }

    pub fn receive_stored_msg(self: Arc<Self>) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => match self.sockets.entry(sock) {
                Entry::Occupied(entry) => {
                    entry.get().receive(
                        match self.stored_msg.read().unwrap().clone() {
                            Some(msg) => msg,
                            None => return Err(DemuxError::MissingContext)
                        },
                    )
                }
                Entry::Vacant(_) => {
                    Err(DemuxError::MissingSession)
                }
            },
            None => Err(DemuxError::MissingSession),
            
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
