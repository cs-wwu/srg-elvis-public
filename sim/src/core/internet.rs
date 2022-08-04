use super::{Machine, Mtu, Network, NetworkId, SharedProtocol};
use std::sync::{Arc, Mutex};

/// The top-level container that controls the simulation.
#[derive(Default)]
pub struct Internet {
    machines: Vec<Machine>,
    networks: Vec<Arc<Mutex<Network>>>,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a network to the simulation and returns a handle to it.
    pub fn network(&mut self, mtu: Mtu) -> NetworkId {
        let id: NetworkId = self.networks.len().try_into().unwrap();
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
                .get(network as usize)
                .unwrap()
                .clone()
                .lock()
                .unwrap()
                .attach(&mut machine)
        }
        self.machines.push(machine);
    }

    /// Runs the simulation.
    pub fn run(self) {
        for network in self.networks {
            network.lock().unwrap().start();
        }
        for mut machine in self.machines {
            machine.start();
        }
    }
}
