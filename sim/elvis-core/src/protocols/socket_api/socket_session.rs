use super::socket::Socket;
use crate::{machine::ProtocolMap, protocol::DemuxError, session::SendError, Message, Session};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

pub(super) struct SocketSession {
    pub upstream: RwLock<Option<Arc<Socket>>>,
    pub downstream: Arc<dyn Session>,
    pub stored_messages: RwLock<VecDeque<Message>>,
}

impl SocketSession {
    pub fn receive(&self, message: Message) -> Result<(), DemuxError> {
        match self.upstream.read().unwrap().clone() {
            Some(sock) => sock.receive(message),
            None => {
                self.stored_messages.write().unwrap().push_back(message);
                Ok(())
            }
        }
    }

    pub fn receive_stored_messages(self: Arc<Self>) -> Result<(), DemuxError> {
        match self.upstream.read().unwrap().clone() {
            Some(sock) => {
                let mut queue = self.stored_messages.write().unwrap();
                while !queue.is_empty() {
                    sock.receive(queue.pop_front().unwrap())?;
                }
                Ok(())
            }
            None => Err(DemuxError::MissingSession),
        }
    }

    pub fn connection_established(self: Arc<Self>) {
        if let Some(sock) = self.upstream.read().unwrap().clone() {
            sock.connection_established();
        }
    }
}

impl Session for SocketSession {
    fn send(&self, message: Message, protocols: ProtocolMap) -> Result<(), SendError> {
        self.downstream.send(message, protocols)
    }
}
