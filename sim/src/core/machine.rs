use super::{protocol::SharedProtocol, NetworkId, ProtocolContext, ProtocolId};
use crate::protocols::tap::{NetworkInfo, Tap};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc::Sender, watch};

/// An identifier for a particular [`Machine`] in the simulation.
pub type MachineId = usize;

pub(super) type ProtocolMap = Arc<HashMap<ProtocolId, SharedProtocol>>;

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines are
/// managed by the [`Internet`](super::Internet) and communicate through
/// [`Network`](super::Network)s. Each machine contains a set of
/// [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub struct Machine {
    id: MachineId,
    protocols: ProtocolMap,
    tap: Arc<Mutex<Tap>>,
}

impl Machine {
    /// Creates a new machine containing the `tap` and other `protocols`.
    pub fn new(protocols: impl IntoIterator<Item = SharedProtocol>, id: MachineId) -> Self {
        let tap = Arc::new(Mutex::new(Tap::new()));
        let mut map = HashMap::new();
        for protocol in protocols
            .into_iter()
            .chain(iter::once(tap.clone() as SharedProtocol))
        {
            let id = protocol.lock().unwrap().id();
            match map.entry(id) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    entry.insert(protocol);
                }
            }
        }
        Self {
            id,
            tap,
            protocols: Arc::new(map),
        }
    }

    pub fn attach(&mut self, info: NetworkInfo, network_id: NetworkId) {
        self.tap.lock().unwrap().attach(info, network_id);
    }

    pub fn id(&self) -> MachineId {
        self.id
    }

    /// Gives the machine time to process incoming messages and
    /// [`awake`](super::Protocol::awake) its protocols.
    pub async fn start(&mut self, shutdown: Sender<()>) {
        let protocol_context = ProtocolContext::new(self.protocols.clone());
        for protocol in self.protocols.values() {
            protocol
                .clone()
                .lock()
                .unwrap()
                .start(protocol_context.clone(), shutdown.clone())
                .await
                .unwrap();
        }
    }
}
