use tokio::sync::mpsc::Sender;

use crate::{machine::ProtocolMap, protocol::DemuxError, session::SendError, Message, Session};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

pub(super) struct SocketSession {
    //pub upstream: RwLock<Option<Arc<Socket>>>,
    pub upstream: RwLock<Option<Sender<Message>>>,
    pub downstream: Arc<dyn Session>,
    pub stored_messages: RwLock<VecDeque<Message>>,
}

impl SocketSession {
    pub fn receive(&self, message: Message) -> Result<(), DemuxError> {
        match self.upstream.read().unwrap().clone() {
            Some(sock) => match sock.try_send(message) {
                Ok(_) => Ok(()),
                Err(_) => Err(DemuxError::Other),
            },
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
                    match sock.try_send(queue.pop_front().unwrap()) {
                        Ok(_) => { },
                        Err(_) => { return Err(DemuxError::Other); },
                    };
                }
                Ok(())
            }
            None => Err(DemuxError::MissingSession),
        }
    }

    pub fn connection_established(self: Arc<Self>) {
        // if let Some(sock) = self.upstream.read().unwrap().clone() {
        //     sock.connection_established();
        // }
        // TODO(giddinl2): Somehow fix this
    }
}

impl Session for SocketSession {
    fn send(&self, message: Message, protocols: ProtocolMap) -> Result<(), SendError> {
        self.downstream.send(message, protocols)
    }
}
