use super::{
    ipv4_parsing::{Ipv4HeaderBuilder, ProtocolNumber},
    Ipv4, LocalAddress, RemoteAddress,
};
use crate::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        tap::{FirstResponder, NetworkId},
        udp::Udp,
    },
    session::SharedSession,
    Session,
};
use std::{error::Error, sync::Arc};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    upstream: ProtocolId,
    /// The session we mux outgoing messages to
    downstream: SharedSession,
    /// The identifying information for this session
    identifier: SessionId,
    /// The ID of the network to send on
    network_id: NetworkId,
}

impl Ipv4Session {
    /// Creates a new IPv4 session
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
    fn send(
        self: Arc<Self>,
        mut message: Message,
        mut context: Context,
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
        self.network_id.apply(&mut context.info);
        FirstResponder::set(&mut context.info, Ipv4::ID.into());
        message.prepend(header);
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
}

/// A set that uniquely identifies a given session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    /// The local address
    pub local: LocalAddress,
    /// The remote address
    pub remote: RemoteAddress,
}