use super::{
    internet::{NetworkHandle, NetworkInfo},
    protocol::{Context, ProtocolId, SharedProtocol},
};
use crate::{network::Delivery, protocols::tap::Tap};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
    sync::Arc,
};
use tokio::sync::{mpsc::Sender, Barrier};

/// An identifier for a particular [`Machine`] in the simulation.
pub(crate) type MachineId = usize;

/// A mapping of protocol IDs to protocols
pub(crate) type ProtocolMap = Arc<HashMap<ProtocolId, SharedProtocol>>;

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
    /// Creates a new machine containing the `tap` and other `protocols`.
    pub fn new(
        protocols: impl IntoIterator<Item = SharedProtocol>,
        id: MachineId,
    ) -> (Self, Sender<Delivery>) {
        let (tap, sender) = Tap::new(id);
        let tap = Arc::new(tap);
        let mut map = HashMap::new();
        for protocol in protocols
            .into_iter()
            .chain(iter::once(tap.clone() as SharedProtocol))
        {
            match map.entry(protocol.clone().id()) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    entry.insert(protocol);
                }
            }
        }
        let machine = Self {
            tap,
            protocols: Arc::new(map),
        };
        (machine, sender)
    }

    pub fn attach(&mut self, network_id: NetworkHandle, info: NetworkInfo) {
        self.tap.clone().attach(network_id, info);
    }

    /// Gives the machine time to process incoming messages and
    /// [`awake`](super::Protocol::awake) its protocols.
    pub fn start(self, shutdown: Sender<()>, initialized: Arc<Barrier>) {
        let protocol_context = Context::new(self.protocols.clone());
        for protocol in self.protocols.values() {
            protocol
                .clone()
                .start(
                    protocol_context.clone(),
                    shutdown.clone(),
                    initialized.clone(),
                )
                .unwrap()
        }
    }

    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }
}
