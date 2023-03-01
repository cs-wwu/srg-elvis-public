use std::sync::Arc;

use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError, SharedSession},
    Message, Session,
};

use super::socket::Socket;

pub(super) struct SocketSession {
    pub upstream: Arc<Socket>,
    pub downstream: SharedSession,
}

impl SocketSession {
    pub fn receive(self: Arc<Self>, message: Message, _context: Context) -> Result<(), DemuxError> {
        /* receive_message_event(
            self.id.local.address,
            self.id.remote.address,
            self.id.local.port,
            self.id.remote.port,
            message.clone(),
        );
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?; */
        match self.upstream.receive(message) {
            Ok(v) => Ok(v),
            Err(_e) => Err(DemuxError::Other),
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
