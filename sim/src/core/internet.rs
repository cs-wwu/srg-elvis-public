//! The [`Internet`] and supporting types.

use super::{
    network::{Attachment, SharedNetwork},
    protocol::SharedProtocol,
    Machine, Network,
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// A unique identifier for a network on an [`Internet`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NetworkHandle(u32);

impl NetworkHandle {
    /// Creates a new network handle.
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }

    /// Gets the raw network ID
    pub(crate) fn into_inner(self) -> u32 {
        self.0
    }
}

/// The top-level container that controls the simulation.
#[derive(Default)]
pub struct Internet {
    machines: Vec<Machine>,
    networks: Vec<NetworkInfo>,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a network to the simulation and returns a handle to it.
    pub fn network(&mut self, network: impl Network + Send + Sync + 'static) -> NetworkHandle {
        let id = self.networks.len();
        self.networks.push(NetworkInfo::new(Arc::new(network)));
        NetworkHandle(id.try_into().unwrap())
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(
        &mut self,
        protocols: impl IntoIterator<Item = SharedProtocol>,
        networks: impl IntoIterator<Item = NetworkHandle>,
    ) {
        let machine_id = self.machines.len();
        let (machine, sender) = Machine::new(protocols, machine_id);
        for network_id in networks.into_iter() {
            let network = self
                .networks
                .get_mut(network_id.into_inner() as usize)
                .unwrap();
            network.attachments.push(Attachment {
                machine: machine_id,
                sender: sender.clone(),
            });
        }
        self.machines.push(machine);
    }

    /// Runs the simulation.
    pub async fn run(mut self) {
        for (network_id, network) in self.networks.into_iter().enumerate() {
            let network = Arc::new(network);
            for attachment in network.clone().attachments.iter() {
                self.machines[attachment.machine].attach(
                    NetworkHandle(network_id.try_into().unwrap()),
                    network.clone(),
                );
            }
        }
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
        for machine in self.machines {
            machine.start(shutdown_sender.clone());
        }
        shutdown_receiver.recv().await.unwrap();
    }
}

/// Information about a network.
#[derive(Clone)]
pub(crate) struct NetworkInfo {
    pub network: SharedNetwork,
    /// The channels to send on corresponding to each machine on the network
    pub attachments: Vec<Attachment>,
}

impl NetworkInfo {
    /// Creates a new network info with no connected machines.
    pub fn new(network: SharedNetwork) -> Self {
        Self {
            network,
            attachments: vec![],
        }
    }
}
