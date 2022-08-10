use super::{
    ipv4_parsing::{Ipv4HeaderBuilder, ProtocolNumber},
    LocalAddress, RemoteAddress,
};
use crate::{
    core::{message::Message, ProtocolContext, ProtocolId, Session, SharedSession},
    protocols::udp::Udp,
};
use std::{error::Error, sync::Arc};

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
        self: Arc<Self>,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let length = message.iter().count();
        let protocol_number = match self.upstream {
            Udp::ID => ProtocolNumber::Udp,
            _ => panic!("Unknown upstream protocol"),
        };
        let header = Ipv4HeaderBuilder::new(
            self.identifier.local.into(),
            self.identifier.remote.into(),
            protocol_number,
            length as u16,
        )
        .build()?;
        let message = message.with_header(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }

    fn start(self: Arc<Self>, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local: LocalAddress,
    pub remote: RemoteAddress,
}
