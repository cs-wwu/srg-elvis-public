use super::{ipv4_parsing::Ipv4HeaderBuilder, Ipv4, Ipv4Address, Recipient};
use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::DemuxError,
    session::{SendError, SharedSession},
    Control, Session, Transport,
};
use std::{
    any::{Any, TypeId},
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    upstream: Transport,
    /// The session we mux outgoing messages to
    downstream: SharedSession,
    /// The identifying information for this session
    id: SessionId,
    /// Inforamation about how and where to send packets
    destination: Recipient,
}

impl Ipv4Session {
    /// Creates a new IPv4 session
    pub(super) fn new(
        downstream: SharedSession,
        upstream: TypeId,
        identifier: SessionId,
        destination: Recipient,
    ) -> Option<Self> {
        Transport::try_from(upstream).ok().map(|upstream| Self {
            upstream,
            downstream,
            id: identifier,
            destination,
        })
    }

    pub fn receive(
        self: Arc<Self>,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        protocols
            .get(self.upstream.into())
            .expect("No such protocol")
            .demux(message, self, control, protocols)?;
        Ok(())
    }
}

impl Session for Ipv4Session {
    #[tracing::instrument(name = "Ipv4Session::send", skip_all)]
    fn send(
        &self,
        mut message: Message,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), SendError> {
        let length = message.iter().count();
        let header = match Ipv4HeaderBuilder::new(
            self.id.local,
            self.id.remote,
            self.upstream as u8,
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
        control.slot = Some(self.destination.slot);
        control.first_responder = Some(TypeId::of::<Ipv4>());
        control.remote.mac = self.destination.mac;
        message.header(header);
        self.downstream.send(message, control, protocols)?;
        Ok(())
    }

    fn info(&self, protocol_id: TypeId) -> Option<Box<dyn Any>> {
        if protocol_id == TypeId::of::<Ipv4>() {
            return Some(Box::new(self.id));
        } else {
            self.downstream.info(protocol_id)
        }
    }
}

impl Debug for Ipv4Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
