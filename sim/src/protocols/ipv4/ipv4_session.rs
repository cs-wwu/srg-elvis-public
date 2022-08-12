use super::{
    ipv4_parsing::{Ipv4HeaderBuilder, ProtocolNumber},
    Ipv4, LocalAddress, RemoteAddress,
};
use crate::{
    core::{
        message::Message,
        protocol::{Context, ProtocolId},
        session::SharedSession,
        Session,
    },
    protocols::{
        tap::{FirstResponder, NetworkId},
        udp::Udp,
    },
};
use std::{error::Error, sync::Arc};

pub struct Ipv4Session {
    upstream: ProtocolId,
    downstream: SharedSession,
    identifier: SessionId,
    network_id: NetworkId,
}

impl Ipv4Session {
    pub(super) fn new(
        downstream: SharedSession,
        upstream: ProtocolId,
        identifier: SessionId,
        network_id: NetworkId,
    ) -> Self {
        Self {
            upstream,
            downstream,
            identifier,
            network_id,
        }
    }
}

impl Session for Ipv4Session {
    fn send(self: Arc<Self>, message: Message, mut context: Context) -> Result<(), Box<dyn Error>> {
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
        self.network_id.apply(&mut context.info);
        FirstResponder::set(&mut context.info, Ipv4::ID.into());
        let message = message.with_header(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }

    fn start(self: Arc<Self>, _context: Context) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    pub local: LocalAddress,
    pub remote: RemoteAddress,
}
