use crate::{logging::machine_creation_event, protocol::SharedProtocol, Id, Shutdown};

use crate::protocols::ipv4::Ipv4Address;
use crate::Network;
use::NetworkAPI;
use crate::protocols::{
    ipv4::Recipients,
    Sockets, Udp, Ipv4, Pci, Dns,
};
use crate::protocols::dns::DnsType;

use rustc_hash::FxHashMap;
use std::{collections::hash_map::Entry, sync::Arc};
use tokio::sync::Barrier;

/// A tap's PCI slot index
pub type PciSlot = u32;

/// A mapping of protocol IDs to protocols
#[derive(Clone)]
pub struct ProtocolMap(Arc<FxHashMap<Id, SharedProtocol>>);

impl ProtocolMap {
    pub fn new(protocols: FxHashMap<Id, SharedProtocol>) -> Self {
        Self(Arc::new(protocols))
    }

    pub fn protocol(&self, id: Id) -> Option<SharedProtocol> {
        self.0.get(&id).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SharedProtocol> {
        self.0.values()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines
/// communicate through [`Network`](super::Network)s. Each machine contains a
/// set of [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub struct Machine {
    pub network_api: NetworkAPI,
    pub protocols: ProtocolMap,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(protocols: impl IntoIterator<Item = SharedProtocol>) -> Machine {
        let mut protocols_map = FxHashMap::default();
        let mut protocol_ids = Vec::new();
        for protocol in protocols.into_iter() {
            match protocols_map.entry(protocol.id()) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    protocol_ids.push(protocol.id());
                    entry.insert(protocol);
                }
            }
        }
        machine_creation_event(protocol_ids);
        Self {
            protocols: ProtocolMap::new(protocols_map),
            network_api: ,
        }
    }


    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new_auth_dns(
        auth_ip: Ipv4Address,
        network: Arc<Network>,
        ip_table: Recipients,
    ) -> Machine {
        let socket_api = Sockets::new(Some(auth_ip)).shared();
        Machine::new([
            socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            Dns::new(DnsType::AUT, auth_ip).shared(),
            // TODO(zachd9757): DnsServer app for doing auth server stuff (wait/listen/etc.)
            // DnsServer::new();
        ])
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub(crate) fn start(self, shutdown: Shutdown, initialized: Arc<Barrier>) {
        for protocol in self.protocols.iter() {
            protocol
                .start(
                    shutdown.clone(),
                    initialized.clone(),
                    self.protocols.clone(),
                )
                .expect("A protocol failed to start")
        }
    }

    /// The number of protocols in the machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }

}
