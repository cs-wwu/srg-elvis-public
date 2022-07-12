use super::{LocalAddress, RemoteAddress};
use crate::{
    core::{message::Message, ControlFlow, ProtocolContext, ProtocolId, Session, SharedSession},
    protocols::udp::Udp,
};
use etherparse::{IpNumber, Ipv4Header};
use std::error::Error;

pub struct Ipv4Session {
    upstream: ProtocolId,
    downstream: SharedSession,
    identifier: SessionId,
}

impl Ipv4Session {
    pub(super) fn new(
        downstream: SharedSession,
        upstream: ProtocolId,
        identifier: SessionId,
    ) -> Self {
        Self {
            upstream,
            downstream,
            identifier,
        }
    }
}

impl Session for Ipv4Session {
    fn send(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let length = message.iter().count();
        let ip_number = match self.upstream {
            Udp::ID => IpNumber::Udp,
            _ => panic!("Unknown upstream protocol"),
        };

        let mut header = Ipv4Header::new(
            length as u16,
            30,
            ip_number,
            self.identifier.local.into_inner().into(),
            self.identifier.remote.into_inner().into(),
        );
        header.header_checksum = header.calc_header_checksum()?;

        let mut header_buffer = vec![];
        header.write(&mut header_buffer)?;

        let message = message.with_header(header_buffer);
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
    pub local: LocalAddress,
    pub remote: RemoteAddress,
}
