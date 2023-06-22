use crate::{
    // control::{ControlError, Key, Primitive},
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocols::ipv4::Ipv4Address,
    protocol::{DemuxError, StartError},
    protocols::pci::Pci,
    Control, Network, Protocol, Shutdown, Session
};

use std::{
    any::TypeId,
    sync::Arc,
};

pub struct DnsSession {
    /// The protocol that we demux incoming messages to
    upstream: TypeId,
    /// The session we mux outgoing messages to
    downstream: Arc<dyn Session>,
    /// The identifying information for this session
    id: SessionId,
}

// impl Session for DnsSession {
//     fn send(self: Arc<Self>, mut message: Message, mut context: Context) -> Result<(), SendError> {

//     }

//     fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {

//     }
// }

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