use super::socket::Socket;
use crate::{
    machine::ProtocolMap,
    protocol::DemuxError,
    session::{SendError, SharedSession},
    Control, Message, Session,
};
use std::{
    any::{Any, TypeId},
    sync::{Arc, RwLock},
};

pub(super) struct SocketSession {
    pub upstream: RwLock<Option<Arc<Socket>>>,
    pub downstream: SharedSession,
    pub stored_msg: RwLock<Option<Message>>,
}

impl SocketSession {
    pub fn receive(&self, message: Message) -> Result<(), DemuxError> {
        match self.upstream.read().unwrap().clone() {
            Some(sock) => sock.receive(message),
            None => Err(DemuxError::MissingSession),
        }
    }

    pub fn receive_stored_msg(self: Arc<Self>) -> Result<(), DemuxError> {
        match self.upstream.read().unwrap().clone() {
            Some(sock) => sock.receive(match self.stored_msg.read().unwrap().clone() {
                Some(msg) => msg,
                None => return Err(DemuxError::MissingContext),
            }),
            None => Err(DemuxError::MissingSession),
        }
    }
}

impl Session for SocketSession {
    fn send(
        &self,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), SendError> {
        self.downstream.send(message, control, protocols)
    }

    fn info(&self, protocol_id: TypeId) -> Option<Box<dyn Any>> {
        self.downstream.info(protocol_id)
    }
}
