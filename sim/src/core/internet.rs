use super::{ControlFlow, Mac, Machine, Message, Network};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub struct Internet {
    machines: Vec<Machine>,
    /// Contains a mapping from a machine index to network indices
    networks_for_machine: HashMap<usize, Rc<Vec<usize>>>,
    networks: Rc<Vec<Rc<RefCell<Network>>>>,
}

impl Internet {
    pub fn new(machines: Vec<Machine>, networks: Vec<Network>) -> Self {
        let networks_for_machine: HashMap<_, _> = (0..machines.len())
            .map(|machine_index| {
                let networks: Vec<_> = networks
                    .iter()
                    .enumerate()
                    .filter_map(|(network_index, network)| {
                        if network
                            .connected_machines()
                            .iter()
                            .any(|&i| i == machine_index)
                        {
                            Some(network_index)
                        } else {
                            None
                        }
                    })
                    .collect();
                (machine_index, Rc::new(networks))
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

pub struct MachineContext {
    mac: Mac,
    /// Contains a mapping from a machine index to network indices
    networks_for_machine: Rc<Vec<usize>>,
    networks: Rc<Vec<Rc<RefCell<Network>>>>,
}

impl MachineContext {
    pub fn networks(&self) -> impl Iterator<Item = Rc<RefCell<Network>>> {
        NetworksIterator {
            current: 0,
            networks_for_machine: self.networks_for_machine.clone(),
            networks: self.networks.clone(),
        }
    }

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

struct NetworksIterator {
    current: usize,
    networks_for_machine: Rc<Vec<usize>>,
    networks: Rc<Vec<Rc<RefCell<Network>>>>,
}

impl Iterator for NetworksIterator {
    type Item = Rc<RefCell<Network>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.networks_for_machine.get(self.current).cloned()?;
        self.current += 1;
        self.networks.get(index).cloned()
    }
}
