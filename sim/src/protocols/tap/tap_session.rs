use super::{tap_misc::TapError, NetworkId};
use crate::core::{message::Message, ProtocolContext, ProtocolId, Session};
use std::{error::Error, mem};

#[derive(Clone)]
pub struct TapSession {
    network: NetworkId,
    outgoing: Vec<Message>,
    upstream: ProtocolId,
}

impl TapSession {
    pub(super) fn new(upstream: ProtocolId, network: NetworkId) -> Self {
        Self {
            upstream,
            network,
            outgoing: vec![],
        }
    }

    pub fn network(&self) -> NetworkId {
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
        let mut protocol = protocol.lock().unwrap();
        protocol.demux(message, context)
    }

    fn start(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    upstream: ProtocolId,
    network: NetworkId,
}

impl SessionId {
    pub fn new(upstream: ProtocolId, network: NetworkId) -> Self {
        Self { upstream, network }
    }
}
