use crate::protocols::ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address};

/// Uniquely identifies the fragments of a particular datagram. See the
/// Identification section for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufId {
    /// The remote IP address
    src: Ipv4Address,
    /// The local IP address
    dst: Ipv4Address,
    /// The transmission protocol used upstream from IP
    protocol: u8,
    /// The identification field of the IP header
    identification: u16,
}

impl BufId {
    /// Gets the segment identifier to a given IP header
    pub fn from_header(header: &Ipv4Header) -> Self {
        Self {
            src: header.source,
            dst: header.destination,
            protocol: header.protocol,
            identification: header.identification,
        }
    }
}
