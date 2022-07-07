use super::{NetworkIndex, Tap};
use crate::core::{Message, ProtocolContext, ProtocolId, RcSession, Session};
use std::{error::Error, mem};

#[derive(Clone)]
pub struct TapSession {
    network: NetworkIndex,
    outgoing: Vec<Message>,
    upstream: ProtocolId,
}

impl TapSession {
    pub(super) fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self {
            upstream,
            network,
            outgoing: vec![],
        }
    }

    pub fn network(&self) -> NetworkIndex {
        self.network
    }

    pub fn outgoing(&mut self) -> Vec<Message> {
        mem::take(&mut self.outgoing)
    }
}

impl Session for TapSession {
    fn protocol(&self) -> ProtocolId {
        Tap::ID
    }

    fn send(
        &mut self,
        _self_handle: RcSession,
        message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let header: [u8; 2] = self.upstream.into();
        let message = message.with_header(&header);
        self.outgoing.push(message);
        Ok(())
    }

    fn recv(
        &mut self,
        _self_handle: RcSession,
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Cannot recv on a Tap")
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
    upstream: ProtocolId,
    network: NetworkIndex,
}

impl SessionId {
    pub fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self { upstream, network }
    }
}
