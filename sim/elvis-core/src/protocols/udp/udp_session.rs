use super::{
    udp_misc::{LocalPort, RemotePort},
    udp_parsing::build_udp_header,
};
use crate::{
    control::{Key, Primitive},
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::ipv4::{LocalAddress, RemoteAddress},
    session::SharedSession,
    Session, logging::{send_message_event, receive_message_event},
};
use std::{error::Error, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: ProtocolId,
    pub downstream: SharedSession,
    pub identifier: SessionId,
}

impl Session for UdpSession {
    fn send(self: Arc<Self>, mut message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        let id = self.identifier;
        let header = build_udp_header(
            self.identifier.local_address.into(),
            id.local_port.into(),
            self.identifier.remote_address.into(),
            id.remote_port.into(),
            message.iter(),
        )?;
        send_message_event(self.identifier.local_address.into(), self.identifier.remote_address.into(), id.local_port.into(), id.remote_port.into(), message.clone());
        message.prepend(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        receive_message_event(self.identifier.local_address.into(), self.identifier.remote_address.into(), self.identifier.local_port.into(), self.identifier.remote_port.into(), message.clone());
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, Box<dyn Error>> {
        self.downstream.clone().query(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local_address: LocalAddress,
    pub local_port: LocalPort,
    pub remote_address: RemoteAddress,
    pub remote_port: RemotePort,
}
