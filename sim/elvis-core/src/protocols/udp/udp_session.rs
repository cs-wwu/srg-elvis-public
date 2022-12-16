use super::udp_parsing::build_udp_header;
use crate::{
    control::{Key, Primitive},
    logging::{receive_message_event, send_message_event},
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::ipv4::Ipv4Address,
    session::SharedSession,
    Session,
};
use std::{fmt::Debug, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: ProtocolId,
    pub downstream: SharedSession,
    pub id: SessionId,
}

impl Session for UdpSession {
    #[tracing::instrument(name = "UdpSession::send", skip(message, context))]
    fn send(self: Arc<Self>, mut message: Message, context: Context) -> Result<(), ()> {
        let id = self.id;
        let header = match build_udp_header(
            self.id.local.address.into(),
            id.local.port.into(),
            self.id.remote.address.into(),
            id.remote.port.into(),
            message.iter(),
        ) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(())?
            }
        };
        send_message_event(
            self.id.local.address.into(),
            self.id.remote.address.into(),
            id.local.port.into(),
            id.remote.port.into(),
            message.clone(),
        );
        message.prepend(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    #[tracing::instrument(name = "UdpSession::receive", skip(message, context))]
    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), ()> {
        receive_message_event(
            self.id.local.address.into(),
            self.id.remote.address.into(),
            self.id.local.port.into(),
            self.id.remote.port.into(),
            message.clone(),
        );
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, ()> {
        self.downstream.clone().query(key)
    }
}

impl Debug for UdpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpSession").field("id", &self.id).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Socket {
    pub address: Ipv4Address,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local: Socket,
    pub remote: Socket,
}
