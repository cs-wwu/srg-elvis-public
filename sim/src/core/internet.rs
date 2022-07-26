use super::{message::Message, ControlFlow, Machine, MachineId, Network};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type MachineIndex = usize;
type NetworkIndex = usize;

/// A shared, mutable handle to a network. We will be handing these out to
/// multiple [`Machine`]s at a time.
type SharedNetwork = Rc<RefCell<Network>>;
/// A shared but immutable list of networks in the simulation. We won't be
/// changing which networks there are, so [`RefCell`] is not needed.
type SharedNetworks = Rc<Vec<SharedNetwork>>;
/// A shared handle to a list of network indices. These are used to track which
/// networks are available to a given [`Machine`].
type NetworkIndices = Rc<Vec<NetworkIndex>>;

/// The top-level container that controls the simulation.
pub struct Internet {
    machines: Vec<Machine>,
    /// Contains a mapping from a machine index to network indices
    networks_for_machine: HashMap<MachineIndex, NetworkIndices>,
    networks: SharedNetworks,
}

impl Internet {
    /// Creates a new internet simulation with the given `machines` and
    /// `networks`
    pub fn new(machines: Vec<Machine>, networks: Vec<Network>) -> Self {
        // Each network contain a list of which machines are attached to it. We
        // also need the opposite, a list of which networks are accessible to
        // each machine. We begin by looping over all machine indices.
        let networks_for_machine: HashMap<_, _> = (0..machines.len())
            .map(|machine_index| {
                // We accumulate a list of which networks are reachable by this
                // machine.
                let networks_indices: Vec<_> = networks
                    .iter()
                    .enumerate()
                    .filter_map(|(network_index, network)| {
                        // Check if the current machine index is one of the
                        // network's connected machines. If so, include the
                        // network in our list.
                        network
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

        let networks: Vec<_> = networks
            .into_iter()
            .map(|network| Rc::new(RefCell::new(network)))
            .collect();

        Self {
            machines,
            networks_for_machine,
            networks: Rc::new(networks),
        }
    }

    /// Runs the simulation.
    pub fn run(&mut self) {
        'outer: loop {
            for (mac, machine) in self.machines.iter_mut().enumerate() {
                let mut context = MachineContext {
                    mac,
                    networks_for_machine: self.networks_for_machine[&mac].clone(),
                    networks: self.networks.clone(),
                };
                match machine.awake(&mut context) {
                    ControlFlow::Continue => {}
                    ControlFlow::EndSimulation => break 'outer,
                }
            }
        }
    }
}

/// A context object to facilitate awaking [`Machine`]s.
///
/// Provides the currently executing machine access to information about its
/// execution environment, such as which networks it is connected to or its
/// pending messages.
pub struct MachineContext {
    mac: MachineId,
    /// Contains a mapping from a machine index to network indices
    networks_for_machine: Rc<Vec<NetworkIndex>>,
    networks: Rc<Vec<Rc<RefCell<Network>>>>,
}

impl MachineContext {
    /// Returns an iterator over the networks reachable by the currently
    /// executing [`Machine`].
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

/// An iterator over networks neighboring the currently executing [`Machine`].
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
