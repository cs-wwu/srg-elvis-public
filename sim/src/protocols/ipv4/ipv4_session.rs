use super::{Ipv4, Ipv4Address};
use crate::{
    core::{message::Message, ProtocolContext, ProtocolId, Session, SharedSession},
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
    fn protocol(&self) -> ProtocolId {
        Ipv4::ID
    }

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
            self.identifier.local.into(),
            self.identifier.remote.into(),
        );
        header.header_checksum = header.calc_header_checksum()?;

        let mut header_buffer = vec![];
        header.write(&mut header_buffer)?;

        let message = message.with_header(header_buffer);
        self.downstream.send(message, context)?;
        Ok(())
    }

    fn recv(
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

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local: Ipv4Address,
    pub remote: Ipv4Address,
}

impl SessionId {
    pub fn new(local: Ipv4Address, remote: Ipv4Address) -> Self {
        Self { local, remote }
    }
}
