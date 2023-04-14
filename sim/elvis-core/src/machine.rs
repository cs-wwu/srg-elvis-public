use crate::{
    gcd::{self, set_protocols, Delivery},
    internet::NetworkHandle,
    network::{Mac, Mtu},
    protocol::SharedProtocol,
    protocols::pci::Pci,
    Id,
};
use rustc_hash::FxHashMap;
use std::{collections::hash_map::Entry, sync::Arc};

/// A tap's PCI slot index
pub type PciSlot = u32;

/// A mapping of protocol IDs to protocols
#[derive(Clone, Default)]
pub(crate) struct ProtocolMap(Arc<FxHashMap<Id, SharedProtocol>>);

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

    #[allow(unused)]
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
    protocols: ProtocolMap,
    pci: Arc<Pci>,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(protocols: impl IntoIterator<Item = SharedProtocol>) -> Machine {
        let mut protocols_map = FxHashMap::default();
        let mut protocol_ids = Vec::new();
        let pci = Pci::new().shared();
        for protocol in protocols
            .into_iter()
            .chain(std::iter::once(pci.clone() as SharedProtocol))
        {
            match protocols_map.entry(protocol.id()) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    protocol_ids.push(protocol.id());
                    entry.insert(protocol);
                }
            }
        }
        Self {
            protocols: ProtocolMap::new(protocols_map),
            pci,
        }
    }

    pub fn connect(&mut self, network: NetworkHandle, mac: Mac, mtu: Mtu) {
        self.pci.connect(network, mac, mtu);
    }

    pub fn receive(&self, delivery: Delivery) {
        gcd::set_protocols(self.protocols.clone());
        self.pci.receive(delivery);
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub(crate) fn start(&self) {
        set_protocols(self.protocols.clone());
        for protocol in self.protocols.iter() {
            protocol.start().expect("A protocol failed to start")
        }
    }

    /// The number of protocols in the machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }
}
