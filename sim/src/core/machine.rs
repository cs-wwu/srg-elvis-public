use super::{
    internet::NetworkInfo, protocol::SharedProtocol, NetworkId, ProtocolContext, ProtocolId,
};
use crate::protocols::tap::{Delivery, Tap};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
    sync::Arc,
};
use tokio::sync::mpsc::Sender;

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
            id,
            tap,
            protocols: Arc::new(map),
        };
        (machine, sender)
    }

    pub fn attach(&mut self, network_id: NetworkId, info: Arc<NetworkInfo>) {
        self.tap.clone().attach(network_id, info);
    }

    pub fn id(&self) -> MachineId {
        self.id
    }

    /// Gives the machine time to process incoming messages and
    /// [`awake`](super::Protocol::awake) its protocols.
    pub fn start(&mut self, shutdown: Sender<()>) {
        let protocol_context = ProtocolContext::new(self.protocols.clone());
        for protocol in self.protocols.values() {
            protocol
                .clone()
                .start(protocol_context.clone(), shutdown.clone())
                .unwrap()
        }
    }
}
