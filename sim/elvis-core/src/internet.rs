use super::Machine;
use crate::{gcd::Gcd, network::Network};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineHandle(pub(crate) usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkHandle(pub(crate) usize);

pub struct Internet {
    machines: Vec<Machine>,
    networks: Vec<Network>,
    threads: usize,
}

impl Internet {
    pub fn new() -> Self {
        Self {
            machines: vec![],
            networks: vec![],
            threads: 1,
        }
    }

    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    pub fn all_threads(mut self) -> Self {
        self.threads = num_cpus::get();
        self
    }

    pub fn connect(&mut self, machine: MachineHandle, network: NetworkHandle) {
        let network_handle = network;
        let network = &mut self.networks[network.0];
        network.connect(machine);
        self.machines[machine.0].connect(
            network_handle,
            network.mac_for_machine(machine),
            network.mtu,
        );
    }

    pub fn add_machine(&mut self, machine: Machine) -> MachineHandle {
        let id = self.machines.len();
        self.machines.push(machine);
        MachineHandle(id)
    }

    pub fn add_network(&mut self, network: Network) -> NetworkHandle {
        let id = self.networks.len();
        self.networks.push(network);
        NetworkHandle(id)
    }

    pub fn run(self) {
        let Self {
            machines,
            networks,
            threads,
        } = self;
        for machine in machines.iter() {
            machine.start();
        }
        let machines = Arc::new(machines);
        let networks = Arc::new(networks);
        let gcd = Gcd::new(threads);
        gcd.start(machines, networks);
    }
}
