use super::{
    internet::NetworkHandle,
    protocol::{ProtocolId, SharedProtocol},
};
use crate::{logging::machine_creation_event, network::Delivery, protocols::tap::Tap};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
    sync::Arc,
};
use tokio::sync::{mpsc::Sender, Barrier};

/// An identifier for a particular [`Machine`] in the simulation.
pub(crate) type MachineId = u64;

/// A mapping of protocol IDs to protocols
#[derive(Clone)]
pub struct ProtocolMap(Arc<HashMap<ProtocolId, SharedProtocol>>);

impl ProtocolMap {
    pub fn new(protocols: HashMap<ProtocolId, SharedProtocol>) -> Self {
        Self(Arc::new(protocols))
    }

    pub fn protocol(&self, id: ProtocolId) -> Option<SharedProtocol> {
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
/// A machine is conceptually a computer attached to the internet. Machines are
/// managed by the [`Internet`](super::Internet) and communicate through
/// [`Network`](super::Network)s. Each machine contains a set of
/// [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub(crate) struct Machine {
    protocols: ProtocolMap,
    tap: Arc<Tap>,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(
        protocols: impl IntoIterator<Item = SharedProtocol>,
        id: MachineId,
    ) -> (Self, Sender<Delivery>) {
        let (tap, sender) = Tap::new(id);
        let tap = Arc::new(tap);
        let mut protocols_map = HashMap::new();
        let mut protocol_ids = Vec::new();
        for protocol in protocols
            .into_iter()
            .chain(iter::once(tap.clone() as SharedProtocol))
        {
            match protocols_map.entry(protocol.clone().id()) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    protocol_ids.push(protocol.clone().id());
                    entry.insert(protocol);
                }
            }
        }
        machine_creation_event(id as usize, protocol_ids);
        let machine = Self {
            tap,
            protocols: ProtocolMap::new(protocols_map),
        };
        (machine, sender)
    }

    /// Attaches the machine to the given network.
    pub fn attach(&mut self, network_id: NetworkHandle, sender: Sender<Delivery>) {
        self.tap.clone().attach(network_id, sender);
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub fn start(self, shutdown: Sender<()>, initialized: Arc<Barrier>) {
        for protocol in self.protocols.iter() {
            protocol
                .clone()
                .start(
                    shutdown.clone(),
                    initialized.clone(),
                    self.protocols.clone(),
                )
                .unwrap()
        }
    }

    /// The number of protocols in the machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }
}
