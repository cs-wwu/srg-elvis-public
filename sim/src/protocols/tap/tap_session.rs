use crate::core::{message::Message, ControlFlow, ProtocolContext, ProtocolId, Session};
use std::{error::Error, mem};

use super::{tap_misc::NetworkIndex, Tap};

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
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Cannot recv on a Tap")
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
