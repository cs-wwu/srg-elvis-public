use super::protocol::{Context, SharedProtocol};
use crate::{id::Id, logging::machine_creation_event};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::{mpsc::Sender, Barrier};

/// A tap's PCI slot index
pub type PciSlot = u32;

/// A mapping of protocol IDs to protocols
pub(crate) type ProtocolMap = Arc<HashMap<Id, SharedProtocol>>;

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines
/// communicate through [`Network`](super::Network)s. Each machine contains a
/// set of [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub struct Machine {
    protocols: ProtocolMap,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(protocols: impl IntoIterator<Item = SharedProtocol>) -> Machine {
        let mut protocols_map = HashMap::new();
        let mut protocol_ids = Vec::new();
        for protocol in protocols.into_iter() {
            match protocols_map.entry(protocol.clone().id()) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    protocol_ids.push(protocol.clone().id());
                    entry.insert(protocol);
                }
            }
        }
        machine_creation_event(protocol_ids);
        Self {
            protocols: Arc::new(protocols_map),
        }
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub(crate) fn start(self, shutdown: Sender<()>, initialized: Arc<Barrier>) {
        let protocol_context = Context::new(self.protocols.clone());
        for protocol in self.protocols.values() {
            protocol
                .clone()
                .start(
                    protocol_context.clone(),
                    shutdown.clone(),
                    initialized.clone(),
                )
                .expect("A protocol failed to start")
        }
    }

    /// The number of protocols in the machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }
}
