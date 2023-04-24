use super::udp_parsing::build_udp_header;
use crate::{
    control::{Key, Primitive},
    id::Id,
    logging::{receive_message_event, send_message_event},
    message::Message,
    protocol::{Context, DemuxError},
    protocols::utility::Socket,
    session::{QueryError, SendError, SharedSession},
    Session,
};
use std::{fmt::Debug, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: Id,
    pub downstream: SharedSession,
    pub id: SessionId,
}

impl UdpSession {
    pub fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), DemuxError> {
        receive_message_event(
            self.id.local.address,
            self.id.remote.address,
            self.id.local.port,
            self.id.remote.port,
            message.clone(),
        );
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }
}

impl Session for UdpSession {
    #[tracing::instrument(name = "UdpSession::send", skip(message, context))]
    fn send(&self, mut message: Message, context: Context) -> Result<(), SendError> {
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
                tracing::error!("{}", e);
                Err(SendError::Header)?
            }
        };
        send_message_event(
            self.id.local.address,
            self.id.remote.address,
            id.local.port,
            id.remote.port,
            message.clone(),
        );
        message.header(header);
        self.downstream.send(message, context)?;
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
