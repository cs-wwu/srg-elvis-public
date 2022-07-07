use super::{Ipv4, Ipv4Address};
use crate::{
    core::{Message, ProtocolContext, ProtocolId, RcSession, Session},
    protocols::udp::Udp,
};
use etherparse::{IpNumber, Ipv4Header};
use std::error::Error;

pub struct Ipv4Session {
    upstream: ProtocolId,
    downstream: RcSession,
    identifier: SessionId,
}

impl Ipv4Session {
    pub(super) fn new(downstream: RcSession, upstream: ProtocolId, identifier: SessionId) -> Self {
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
        _self_handle: RcSession,
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
        self.downstream
            .borrow_mut()
            .send(self.downstream.clone(), message, context)?;
        Ok(())
    }

    fn recv(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)?
            .borrow_mut()
            .demux(message, self_handle, context)?;
        Ok(())
    }

    fn awake(
        &mut self,
        _self_handle: RcSession,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
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
