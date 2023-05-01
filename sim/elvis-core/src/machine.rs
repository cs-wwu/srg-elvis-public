use crate::{logging::machine_creation_event, protocol::SharedProtocol, Id, Shutdown};
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
        }
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
