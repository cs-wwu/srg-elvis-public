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
    pub stored_msg: RwLock<Option<Message>>,
    pub stored_cxt: RwLock<Option<Context>>,
}

impl SocketSession {
    pub fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => self
                .socket_api
                .clone()
                .forward_to_socket(sock, message, context),
            None => {
                *self.stored_msg.write().unwrap() = Some(message);
                *self.stored_cxt.write().unwrap() = Some(context);
                Ok(())
            }
        }
    }

    pub fn receive_stored_msg(self: Arc<Self>) -> Result<(), DemuxError> {
        match *self.upstream.read().unwrap() {
            Some(sock) => self.socket_api.clone().forward_to_socket(
                sock,
                match self.stored_msg.read().unwrap().clone() {
                    Some(msg) => msg,
                    None => return Err(DemuxError::MissingContext)
                },
                match self.stored_cxt.read().unwrap().clone() {
                    Some(cxt) => cxt,
                    None => return Err(DemuxError::MissingContext)
                },
            ),
            None => Err(DemuxError::MissingSession),
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
