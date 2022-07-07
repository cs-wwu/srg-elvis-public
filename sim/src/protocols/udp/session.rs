use super::Udp;
use crate::{
    core::{Message, ProtocolContext, ProtocolId, RcSession, Session},
    protocols::ipv4::Ipv4Address,
};
use etherparse::UdpHeader;
use std::{cell::RefCell, error::Error, rc::Rc};

pub struct UdpSession {
    upstream: ProtocolId,
    downstream: RcSession,
    identifier: SessionId,
}

impl UdpSession {
    pub(super) fn new(upstream: ProtocolId, downstream: RcSession, identifier: SessionId) -> Self {
        Self {
            upstream,
            downstream,
            identifier,
        }
    }

    pub(super) fn new_shared(
        upstream: ProtocolId,
        downstream: RcSession,
        identifier: SessionId,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(upstream, downstream, identifier)))
    }
}

impl Session for UdpSession {
    fn protocol(&self) -> ProtocolId {
        Udp::ID
    }

    fn send(
        &mut self,
        _self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let id = self.identifier;
        let payload_len = message.iter().count();
        // Todo: We want to use the checksum
        let header = UdpHeader::without_ipv4_checksum(id.local_port, id.remote_port, payload_len)?;
        let mut header_bytes = vec![];
        header.write(&mut header_bytes)?;
        let message = message.with_header(header_bytes);
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
    pub local_address: Ipv4Address,
    pub local_port: u16,
    pub remote_address: Ipv4Address,
    pub remote_port: u16,
}
