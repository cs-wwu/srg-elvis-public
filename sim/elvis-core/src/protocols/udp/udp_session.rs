use super::{
    udp_misc::{LocalPort, RemotePort},
    udp_parsing::build_udp_header,
};
use crate::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::ipv4::{LocalAddress, RemoteAddress},
    session::SharedSession,
    Session,
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
        message.prepend(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local_address: LocalAddress,
    pub local_port: LocalPort,
    pub remote_address: RemoteAddress,
    pub remote_port: RemotePort,
}
