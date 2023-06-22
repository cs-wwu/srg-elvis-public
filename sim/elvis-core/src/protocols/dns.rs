//! An implementation of the Domain Name Structure

pub mod dns_session;

use crate::{
    control::{ControlError, Key, Primitive},
    Id,
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocols::ipv4::Ipv4Address,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::pci::Pci,
    protocols::dns::dns_session::{DnsSession, SessionId},
    session::SharedSession,
    Control, Network, Protocol, Shutdown, Session,
};

use {
    dashmap::{mapref::entry::Entry, DashMap},
    std::sync::Arc,
    std::collections::HashMap,
    tokio::sync::Barrier,
};

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to IP mappings.
pub struct Dns {
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    name_to_ip: DashMap<String, Ipv4Address>,

    /// The type for this Dns protocol telling us what kind of machine we're on
    dns_type: DnsType,

    /// Direct reference to Sockets
    // TODO(zachd9757): Replace this with a reference to the Network API once it exists
    // sockets: Sockets,

    /// Well-known IP for the authoritative server
    auth_ip: Ipv4Address,
}

impl Dns {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(16);  // 16 is the unique ID for DNS

    /// Creates a new instance of the protocol.
    pub fn new(dns_type: DnsType, auth_ip: Ipv4Address) -> Self {
        Self {
            name_to_ip: DashMap::new(),
            dns_type,
            auth_ip,
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Adds a new mapping to the name_to_ip cache.
    pub fn add_mapping(&self, name: String, ip: Ipv4Address) {
        self.name_to_ip.insert(name, ip);
    }

    /// Checks local name_to_ip cache for ['Ipv4Address'] given a name.
    pub fn get_mapping(&self, name: String) -> Result<Ipv4Address, DnsError> {
        match self.name_to_ip.entry(name) {
            Entry::Occupied(e) => {
                Ok(e.get().clone())
            }
            Entry::Vacant(e) => {
                Err(DnsError::Cache)
            }
        }
    }

    /// Finds the IP associated with the given domain name.
    fn get_host_by_name(
        &self,
        name: String,
        protocols: ProtocolMap,
    ) -> Result<Ipv4Address, /* SocketError */ DnsError> {
        // Get DNS protocol from this socket protocol's machine
        // let dns: Dns = match protocols.protocol(Dns::ID) {
        //     Some(p) => p,
        //     None => {
        //         return Err(SocketError::Other);
        //     }
        // };

        match self.get_mapping(name) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(DnsError) => {
                // TODO(zachd9757): Check authoritative server
                Err(/* SocketError::Other*/ DnsError::Other)
            },
        }
    }
}

impl Protocol for Dns {
    fn id(&self) -> Id {
        Self::ID
    }

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        //TODO
        Err(StartError::Other)
    }

    fn open(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        //TODO
        Err(OpenError::Other)
    }

    fn listen(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        //TODO
        Err(ListenError::Other)
    }

    fn demux(
        &self,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        //TODO
        Err(DemuxError::Other)
    }

    fn query(
        &self,
        key: Key
    ) -> Result<Primitive, QueryError> {
        //TODO
        Err(QueryError::NonexistentKey)
    }
}

pub enum DnsType {
    /// Authoritative Server
    AUT,
    /// Client
    CLI,
    /// Recursive
    REC,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Checks HashMap functionality
    fn add_and_lookup_mapping() {
        // Initialize struct
        let dns: Dns = Dns::new(DnsType::CLI, Ipv4Address::CURRENT_NETWORK);

        // Create and add mapping
        let name: String = String::from("Name");
        let ip: Ipv4Address = Ipv4Address::CURRENT_NETWORK;
        dns.add_mapping(name.clone(), ip);

        // Verify that lookup matches what was added
        let check = dns.get_mapping(name);
        assert_eq!(Ok(ip), check);
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsError {
    #[error("DNS cache lookup error")]
    Cache,
    #[error("Unspecified DNS error")]
    Other,
}