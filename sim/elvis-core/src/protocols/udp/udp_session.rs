use super::udp_parsing::build_udp_header;
use crate::{
    control::{Key, Primitive},
    gcd::get_protocol,
    id::Id,
    message::Message,
    protocol::DemuxError,
    protocols::utility::Socket,
    session::{QueryError, SendError, SharedSession},
    Control, Session,
};
use std::{fmt::Debug, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: Id,
    pub downstream: SharedSession,
    pub id: SessionId,
}

impl UdpSession {
    pub fn receive(self: Arc<Self>, message: Message, control: Control) -> Result<(), DemuxError> {
        get_protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, control)?;
        Ok(())
    }
}

impl Session for UdpSession {
    fn send(&self, mut message: Message, control: Control) -> Result<(), SendError> {
        let id = self.id;
        // TODO(hardint): Should this fail or just segment the message into
        // multiple IP packets?
        let header = match build_udp_header(
            self.id.local.address,
            id.local.port,
            self.id.remote.address,
            id.remote.port,
            message.iter(),
            message.len(),
        ) {
            Ok(header) => header,
            Err(e) => {
                eprintln!("{}", e);
                Err(SendError::Header)?
            }
        };
        message.header(header);
        self.downstream.send(message, control)?;
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.query(key)
    }
}

impl Debug for UdpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpSession").field("id", &self.id).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local: Socket,
    pub remote: Socket,
}

impl SessionId {
    pub fn new(local: Socket, remote: Socket) -> Self {
        Self { local, remote }
    }
}
