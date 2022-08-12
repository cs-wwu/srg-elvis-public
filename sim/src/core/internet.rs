use super::{Machine, MachineId, SharedProtocol};
use crate::protocols::tap::Delivery;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};

pub type NetworkIndex = u32;
pub type Mtu = u32;

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
    pub fn network(&mut self, mtu: Mtu) -> NetworkIndex {
        let id = self.networks.len();
        self.networks.push(Network::new(mtu));
        id as NetworkIndex
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(
        &mut self,
        protocols: impl IntoIterator<Item = SharedProtocol>,
        networks: impl IntoIterator<Item = NetworkIndex>,
    ) {
        let machine_id = self.machines.len();
        let (machine, sender) = Machine::new(protocols, machine_id);
        for network_id in networks.into_iter() {
            let network = self.networks.get_mut(network_id as usize).unwrap();
            network.machines.push(machine_id);
            network.info.senders.push(sender.clone());
        }
        self.machines.push(machine);
    }

    /// Runs the simulation.
    pub async fn run(mut self) {
        // TODO(hardint): Parallelize
        for (network_id, network) in self.networks.into_iter().enumerate() {
            for machine_id in network.machines {
                self.machines[machine_id]
                    .attach(network_id as NetworkIndex, Arc::new(network.info.clone()))
            }
        }
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
        // TODO(hardint): Maybe parallelize?
        for mut machine in self.machines {
            machine.start(shutdown_sender.clone());
        }
        shutdown_receiver.recv().await.unwrap();
    }
}

#[derive(Clone)]
pub struct NetworkInfo {
    pub mtu: Mtu,
    pub senders: Vec<Sender<Delivery>>,
}

impl NetworkInfo {
    pub fn new(mtu: Mtu) -> Self {
        Self {
            mtu,
            senders: vec![],
        }
    }
}

#[derive(Clone)]
struct Network {
    pub info: NetworkInfo,
    pub machines: Vec<MachineId>,
}

impl Network {
    pub fn new(mtu: Mtu) -> Self {
        Self {
            info: NetworkInfo::new(mtu),
            machines: vec![],
        }
    }
}
