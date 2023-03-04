use std::sync::{Arc, RwLock};

use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError, SharedSession},
    Id, Message, Session,
};

use super::Sockets;

pub(super) struct SocketSession {
    pub upstream: RwLock<Option<Id>>,
    pub downstream: SharedSession,
    pub socket_api: Arc<Sockets>,
}

impl SocketSession {
    pub fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => self
                .socket_api
                .clone()
                .forward_to_socket(sock, message, context),
            None => Ok(()),
        }
    }
}

impl Session for SocketSession {
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), SendError> {
        self.downstream.clone().send(message, context)
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}
