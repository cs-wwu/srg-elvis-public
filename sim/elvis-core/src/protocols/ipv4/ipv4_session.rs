use super::{ipv4_parsing::Ipv4HeaderBuilder, Ipv4, Ipv4Address, Recipient};
use crate::{
    control::{Key, Primitive},
    gcd::get_protocol,
    id::Id,
    message::Message,
    network::Network,
    protocol::DemuxError,
    protocols::pci::Pci,
    session::{QueryError, SendError, SharedSession},
    Control, Session,
};
use std::{fmt::Debug, sync::Arc};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    upstream: Id,
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
        upstream: Id,
        identifier: SessionId,
        destination: Recipient,
    ) -> Self {
        Self {
            upstream,
            downstream,
            id: identifier,
            destination,
        }
    }

    pub fn receive(self: Arc<Self>, message: Message, control: Control) -> Result<(), DemuxError> {
        get_protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, control)?;
        Ok(())
    }
}

impl Session for Ipv4Session {
    fn send(&self, mut message: Message, mut control: Control) -> Result<(), SendError> {
        let length = message.iter().count();
        let header = match Ipv4HeaderBuilder::new(
            self.id.local,
            self.id.remote,
            self.upstream.into_inner() as u8,
            length as u16,
        )
        .build()
        {
            Ok(header) => header,
            Err(e) => {
                eprintln!("{}", e);
                Err(SendError::Header)?
            }
        };
        Pci::set_pci_slot(self.destination.slot, &mut control);
        Network::set_protocol(Ipv4::ID, &mut control);
        if let Some(mac) = self.destination.mac {
            Network::set_destination(mac, &mut control);
        }
        message.header(header);
        self.downstream.send(message, control)?;
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.query(key)
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
