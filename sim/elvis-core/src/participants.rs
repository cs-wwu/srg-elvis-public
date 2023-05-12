use crate::{machine::PciSlot, network::Mac, protocols::ipv4::Ipv4Address, Id};
use std::ops::{Deref, DerefMut};

/// A key-value store with which to exchange data between protocols.
///
/// [`Protocol`](super::Protocol)s often need to pass information to one another
/// such as lists of participants, information extracted from headers, and
/// configuration for opening a session. A control facilitates passing such
/// information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Participants {
    /// Box the contents of the control so that it is less expensive to pass to function parameters
    inner: Box<ParticipantsInner>,
}

/// Control information for incoming or outgoing packets
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParticipantsInner {
    /// The PCI slot that will be sent on or that was received from
    pub slot: Option<PciSlot>,
    /// The protocol that PCI will forward incoming messages to
    pub first_responder: Option<Id>,
    /// Information about the local connection endpoint
    pub local: Endpoint,
    /// Information about the remote connection endpoint
    pub remote: Endpoint,
}

/// Specifies information about a connection endpoint
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Endpoint {
    /// The MAC address
    pub mac: Option<Mac>,
    /// The IPv4 address
    pub address: Option<Ipv4Address>,
    /// The UDP or TCP port
    pub port: Option<u16>,
}

impl Participants {
    /// Creates a new control.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Deref for Participants {
    type Target = ParticipantsInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Participants {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
