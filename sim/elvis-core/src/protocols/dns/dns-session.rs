

pub struct DnsSesion {
    /// The protocol that we demux incoming messages to
    upstream: Id,
    /// The session we mux outgoing messages to
    downstream: SharedSession,
    /// The identifying information for this session
    id: SessionId,
}

impl Session for DnsSession {

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