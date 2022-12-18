use super::{
    ipv4_parsing::{Ipv4HeaderBuilder, ProtocolNumber},
    Ipv4, Ipv4Address,
};
use crate::{
    control::{Key, Primitive},
    machine::TapSlot,
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{pci::Pci, udp::Udp},
    session::{QueryError, ReceiveError, SendError, SharedSession},
    Session,
};
use std::{fmt::Debug, sync::Arc};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    upstream: ProtocolId,
    /// The session we mux outgoing messages to
    downstream: SharedSession,
    /// The identifying information for this session
    id: SessionId,
    /// The PCI slot to send on
    tap_slot: TapSlot,
}

impl Ipv4Session {
    /// Creates a new IPv4 session
    pub(super) fn new(
        downstream: SharedSession,
        upstream: ProtocolId,
        identifier: SessionId,
        tap_slot: TapSlot,
    ) -> Self {
        Self {
            upstream,
            downstream,
            id: identifier,
            tap_slot,
        }
    }
}

impl Session for Ipv4Session {
    #[tracing::instrument(name = "Ipv4Session::send", skip(message, context))]
    fn send(self: Arc<Self>, mut message: Message, mut context: Context) -> Result<(), SendError> {
        let length = message.iter().count();
        let protocol_number = match self.upstream {
            Udp::ID => ProtocolNumber::Udp,
            _ => panic!("Unknown upstream protocol"),
        };
        let header = match Ipv4HeaderBuilder::new(
            self.id.local,
            self.id.remote,
            protocol_number,
            length as u16,
        )
        .build()
        {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(SendError::Header)?
            }
        };
        Pci::set_tap_index(self.tap_slot, &mut context.info);
        Pci::set_first_responder(Ipv4::ID, &mut context.info);
        message.prepend(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    #[tracing::instrument(name = "Ipv4Session::receive", skip_all)]
    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), ReceiveError> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}

impl Debug for Ipv4Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ipv4Session")
            .field("identifier", &self.id)
            .finish()
    }
}

/// A set that uniquely identifies a given session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    /// The local address
    pub local: Ipv4Address,
    /// The remote address
    pub remote: Ipv4Address,
}

impl SessionId {
    pub fn new(local: Ipv4Address, remote: Ipv4Address) -> Self {
        Self { local, remote }
    }
}
