use super::{message::Message, ControlFlow, Machine, MachineId, Mtu, Network, RcProtocol};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type NetworkIndex = usize;

/// A shared, mutable handle to a network. We will be handing these out to
/// multiple machines at a time.
type SharedNetwork = Rc<RefCell<Network>>;
/// A shared but immutable list of networks in the simulation. We will not be
/// mutating the vector after creation to no RefCell is needed.
type SharedNetworks = Rc<Vec<SharedNetwork>>;
/// A shared handle to a list of network indices. These are used to track which
/// networks are available to a given machine.
type NetworkIndices = Rc<Vec<NetworkIndex>>;

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
    pub fn network(&mut self, mtu: Mtu) -> NetworkIndex {
        self.networks.push(Rc::new(RefCell::new(Network::new(mtu))));
        self.networks.len() - 1
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(
        &mut self,
        protocols: impl IntoIterator<Item = RcProtocol>,
        networks: impl IntoIterator<Item = NetworkIndex>,
    ) {
        let mut machine = Machine::new(protocols, self.machines.len());
        for network in networks.into_iter() {
            let network = self.networks.get(network).unwrap();
            network.borrow_mut().attach(&machine);
            machine.attach(network.borrow());
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
                            .borrow()
                            .connected_machines()
                            .iter()
                            .any(|&connected| connected == machine_index)
                            .then_some(network_index)
                    })
                    .collect();
                // The key-value pair to store in the map
                (machine_index, Rc::new(networks_indices))
            })
            .collect();

        networks_for_machine
    }

    /// Runs the simulation.
    pub fn run(mut self) {
        let networks_for_machine = self.networks_for_machine();
        let networks = Rc::new(self.networks);
        'outer: loop {
            for (mac, machine) in self.machines.iter_mut().enumerate() {
                let mut context = MachineContext {
                    mac,
                    networks_for_machine: networks_for_machine[&mac].clone(),
                    networks: networks.clone(),
                };
                match machine.awake(&mut context) {
                    ControlFlow::Continue => {}
                    ControlFlow::EndSimulation => break 'outer,
                }
            }
        }
    }
}

/// A context object to facilitate awaking machines.
///
/// Provides the currently executing machine access to information about its
/// execution environment, such as which networks it is connected to or its
/// pending messages.
pub struct MachineContext {
    mac: MachineId,
    /// Contains a mapping from a machine index to network indices
    networks_for_machine: Rc<Vec<NetworkIndex>>,
    networks: SharedNetworks,
}

impl MachineContext {
    /// Returns an iterator over the networks reachable by the currently
    /// executing machine.
    pub fn networks(&self) -> impl Iterator<Item = Rc<RefCell<Network>>> {
        NetworksIterator {
            current: 0,
            networks_for_machine: self.networks_for_machine.clone(),
            networks: self.networks.clone(),
        }
    }

    /// Returns a list of the messages queued for delivery to the currently
    /// executing machine from all of its connected networks.
    pub fn pending(&self) -> Vec<Message> {
        let mut networks = self.networks();
        let mut messages = if let Some(network) = networks.next() {
            network.borrow_mut().take_queue(self.mac)
        } else {
            vec![]
        };

        for network in networks {
            messages.append(&mut network.borrow_mut().take_queue(self.mac));
        }

        messages
    }
}

/// An iterator over networks neighboring the currently executing machine.
struct NetworksIterator {
    current: NetworkIndex,
    networks_for_machine: NetworkIndices,
    networks: SharedNetworks,
}

impl Iterator for NetworksIterator {
    type Item = Rc<RefCell<Network>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.networks_for_machine.get(self.current).cloned()?;
        self.current += 1;
        self.networks.get(index).cloned()
    }
}
