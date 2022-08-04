use super::{Machine, MachineId, Mtu, Network, NetworkId, SharedProtocol};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// A shared, mutable handle to a network. We will be handing these out to
/// multiple machines at a time.
type SharedNetwork = Arc<Mutex<Network>>;
/// A shared but immutable list of networks in the simulation. We will not be
/// mutating the vector after creation to no Mutex is needed.
type SharedNetworks = Arc<Vec<SharedNetwork>>;
/// A shared handle to a list of network indices. These are used to track which
/// networks are available to a given machine.
type NetworkIndices = Arc<Vec<NetworkId>>;

/// The top-level container that controls the simulation.
#[derive(Default)]
pub struct Internet {
    machines: Vec<Machine>,
    networks: Vec<SharedNetwork>,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a network to the simulation and returns a handle to it.
    pub fn network(&mut self, mtu: Mtu) -> NetworkId {
        let id = self.networks.len();
        self.networks
            .push(Arc::new(Mutex::new(Network::new(id, mtu))));
        id
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(
        &mut self,
        protocols: impl IntoIterator<Item = SharedProtocol>,
        networks: impl IntoIterator<Item = NetworkId>,
    ) {
        let mut machine = Machine::new(protocols, self.machines.len());
        for network in networks.into_iter() {
            self.networks
                .get(network)
                .unwrap()
                .clone()
                .lock()
                .unwrap()
                .attach(&mut machine)
        }
        self.machines.push(machine);
    }

    /// Creates a new internet simulation with the given `machines` and
    /// `networks`
    fn networks_for_machine(&self) -> HashMap<MachineId, NetworkIndices> {
        // Each network contain a list of which machines are attached to it. We
        // also need the opposite, a list of which networks are accessible to
        // each machine. We begin by looping over all machine indices.
        let networks_for_machine: HashMap<_, _> = (0..self.machines.len())
            .map(|machine_index| {
                // We accumulate a list of which networks are reachable by this
                // machine.
                let networks_indices: Vec<_> = self
                    .networks
                    .iter()
                    .enumerate()
                    .filter_map(|(network_index, network)| {
                        // Check if the current machine index is one of the
                        // network's connected machines. If so, include the
                        // network in our list.
                        network
                            .lock()
                            .unwrap()
                            .connected_machines()
                            .any(|&connected| connected == machine_index)
                            .then_some(network_index)
                    })
                    .collect();
                // The key-value pair to store in the map
                (machine_index, Arc::new(networks_indices))
            })
            .collect();

        networks_for_machine
    }

    /// Runs the simulation.
    pub fn run(mut self) {
        let networks_for_machine = self.networks_for_machine();
        let networks = Arc::new(self.networks);
        for (mac, machine) in self.machines.iter_mut().enumerate() {
            machine.start();
        }
    }
}
