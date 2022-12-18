//! The [`Internet`] and supporting types.

use super::{protocol::SharedProtocol, Machine};
use std::sync::Arc;
use tokio::sync::{mpsc, Barrier};

/// The top-level container that controls the simulation.
#[derive(Default)]
pub struct Internet {
    /// The machines attached to the internet
    machines: Vec<Machine>,
    /// The total number of protocols in all the machines on the internet
    protocol_count: usize,
}

impl Internet {
    /// Creates a new internet.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a machine to the simulation with the given protocols and attached
    /// to the given networks.
    pub fn machine(&mut self, protocols: impl IntoIterator<Item = SharedProtocol>) {
        let machine = Machine::new(protocols);
        self.protocol_count += machine.protocol_count();
        self.machines.push(machine);
    }

    /// Runs the simulation.
    pub async fn run(self) {
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
