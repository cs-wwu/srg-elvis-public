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
    Control, Network, Protocol, Shutdown, Session
};

use {
    dashmap::{mapref::entry::Entry, DashMap},
    std::sync::Arc,
    std::collections::HashMap
};

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to IP mappings.
pub struct Dns {
    listen_bindings: DashMap<Ipv4Address, Id>,
    sessions: DashMap<SessionId, Arc<DnsSession>>,
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    name_to_ip: HashMap<&str, Ipv4Address>,
}

impl Dns {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::from_string("DNS");

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            name_to_ip: HashMap::new(),
            listen_bindings: Default::default(),
            sessions: Default::default(),
        }
    }

    /// Adds a new mapping to the name_to_ip cache.
    pub fn add_mapping(&self, name: &str, ip: Ipv4Address) {
        self.name_to_ip.insert(name, ip);
    }

    /// Checks local name_to_ip cache for ['Ipv4Address'] given a name.
    pub fn map_lookup(&self, name: &str) -> Ipv4Address {
        self.name_to_ip[name];
    }
}

impl Protocol for Dns {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        //TODO
    }

    fn listen(
        self: Arc<Self>,
        upstream: Id,
        listen_bindings: Default::default(),
        sessions: Default::default(),
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        //TODO
    }

    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        //TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Checks HashMap functionality
    fn add_and_lookup_mapping() {
        println!("Running DNS Test 1");
        let dns: Dns = Dns::new(); // Initialize struct

        // Create and add mapping
        let name: &str = "Name";
        let ip: Ipv4Address = Ipv4Address::new();
        dns.add_mapping(name, ip);

        // Verify that lookup matches what was added
        let check: Ipv4Address = dns.map_lookup(name);
        assert_eq!(ip, check);
    }

    #[test]
    fn random_test() {
        assert_eq!(1, 1);
    }
}