use super::{tap_misc::TapError, NetworkIndex};
use crate::core::{message::Message, ControlFlow, ProtocolContext, ProtocolId, Session};
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
    fn send(
        &mut self,
        message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let message = message.with_header(&self.upstream.into_inner().to_be_bytes());
        self.outgoing.push(message);
        Ok(())
    }

    fn receive(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let protocol = context
            .protocol(self.upstream)
            .ok_or(TapError::NoSuchProtocol(self.upstream))?;
        let mut protocol = protocol.borrow_mut();
        protocol.demux(message, context)
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
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
