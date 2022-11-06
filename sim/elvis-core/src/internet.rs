//! The [`Internet`] and supporting types.

use crate::machine::MachineId;

use super::{network::Attachment, protocol::SharedProtocol, Machine, Network};
use std::sync::Arc;
use tokio::sync::{mpsc, Barrier};

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
    /// The machines attached to the internet
    machines: Vec<Machine>,
    /// The networks participating in the internet
    networks: Vec<NetworkInfo>,
    /// The total number of protocols in all the machines on the internet
    protocol_count: usize,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a network to the simulation and returns a handle to it.
    pub fn network(&mut self, network: impl Network + 'static) -> NetworkHandle {
        let id = self.networks.len();
        self.networks.push(NetworkInfo::new(network));
        NetworkHandle(id.try_into().unwrap())
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(
        &mut self,
        protocols: impl IntoIterator<Item = SharedProtocol>,
        networks: impl IntoIterator<Item = NetworkHandle>,
    ) {
        let machine_id = self.machines.len() as MachineId;
        let (machine, sender) = Machine::new(protocols, machine_id);
        self.protocol_count += machine.protocol_count();
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
        for (network_id, network_info) in self.networks.into_iter().enumerate() {
            let NetworkInfo {
                network,
                attachments,
            } = network_info;
            let attachments: Arc<[_]> = attachments.into();
            let sender = network.start(attachments.clone());
            for attachment in attachments.iter() {
                self.machines[attachment.machine as usize].attach(
                    NetworkHandle(network_id.try_into().unwrap()),
                    sender.clone(),
                );
            }
        }
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
        let initialized = Arc::new(Barrier::new(self.protocol_count));
        for machine in self.machines {
            machine.start(shutdown_sender.clone(), initialized.clone());
        }
        // TODO(hardint): We need to tell all tasks to shut down and wait for
        // them here before proceeding.
        shutdown_receiver.recv().await.unwrap();
    }
}

/// Information about a network.
struct NetworkInfo {
    pub network: Box<dyn Network>,
    /// The channels to send on corresponding to each machine on the network
    pub attachments: Vec<Attachment>,
}

impl NetworkInfo {
    /// Creates a new network info with no connected machines.
    pub fn new(network: impl Network + 'static) -> Self {
        Self {
            network: Box::new(network),
            attachments: vec![],
        }
    }
}
