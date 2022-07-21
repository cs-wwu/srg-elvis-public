use super::udp_misc::{LocalPort, RemotePort};
use crate::{
    core::{message::Message, ControlFlow, ProtocolContext, ProtocolId, Session, SharedSession},
    protocols::ipv4::{LocalAddress, RemoteAddress},
};
use etherparse::UdpHeader;
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
        let payload: Vec<_> = message.iter().collect();
        let ipv4_header = etherparse::Ipv4Header::new(
            payload.len().try_into()?,
            30,
            etherparse::IpNumber::Udp,
            self.identifier.local_address.into(),
            self.identifier.remote_address.into(),
        );
        let header = UdpHeader::with_ipv4_checksum(
            id.local_port.into(),
            id.remote_port.into(),
            &ipv4_header,
            payload.as_slice(),
        )?;
        let mut header_bytes = vec![];
        header.write(&mut header_bytes)?;
        let message = message.with_header(header_bytes);
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
            .borrow_mut()
            .demux(message, context)?;
        Ok(())
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local_address: LocalAddress,
    pub local_port: LocalPort,
    pub remote_address: RemoteAddress,
    pub remote_port: RemotePort,
}
