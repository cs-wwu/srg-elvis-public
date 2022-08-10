use tokio::sync::mpsc;

use super::{Machine, Mtu, Network, SharedProtocol};

pub type NetworkId = u32;

/// The top-level container that controls the simulation.
#[derive(Default)]
pub struct Internet {
    machines: Vec<Machine>,
    networks: Vec<Network>,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a network to the simulation and returns a handle to it.
    pub fn network(&mut self, mtu: Mtu) -> NetworkId {
        let id: NetworkId = self.networks.len().try_into().unwrap();
        self.networks.push(Network::new(id, mtu));
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
                .get_mut(network as usize)
                .unwrap()
                .attach(&mut machine);
        }
        self.machines.push(machine);
    }

    /// Runs the simulation.
    pub async fn run(self) {
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
        for mut network in self.networks.into_iter() {
            network.start(shutdown_sender.clone());
        }
        for mut machine in self.machines {
            machine.start(shutdown_sender.clone());
        }
        shutdown_receiver.recv().await.unwrap();
    }
}
