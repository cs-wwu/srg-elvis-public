use super::{
    udp_misc::{LocalPort, RemotePort},
    udp_parsing::build_udp_header,
};
use crate::{
    core::{message::Message, ProtocolContext, ProtocolId, Session, SharedSession},
    protocols::ipv4::{LocalAddress, RemoteAddress},
};
use std::error::Error;

pub(super) struct UdpSession {
    pub upstream: ProtocolId,
    pub downstream: SharedSession,
    pub identifier: SessionId,
}

impl Session for UdpSession {
    fn send(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let id = self.identifier;
        let header = build_udp_header(
            self.identifier.local_address.into(),
            id.local_port.into(),
            self.identifier.remote_address.into(),
            id.remote_port.into(),
            message.iter(),
        )?;
        let message = message.with_header(header);
        self.downstream.send(message, context)?;
        Ok(())
    }

    fn receive(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .lock()
            .unwrap()
            .demux(message, context)?;
        Ok(())
    }

    fn start(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
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
