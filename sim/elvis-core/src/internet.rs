use super::Machine;
use crate::network::OpaqueNetwork;
use std::sync::Arc;
use tokio::sync::{mpsc, Barrier};

pub async fn run_internet(machines: Vec<Machine>, networks: Vec<OpaqueNetwork>) {
    let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();
    let initialized = Arc::new(Barrier::new(total_protocols + networks.len()));
    for machine in machines {
        machine.start(shutdown_sender.clone(), initialized.clone());
    }
    for network in networks {
        network.start(initialized.clone());
    }
    // TODO(hardint): We need to tell all tasks to shut down and wait for
    // them here before proceeding.
    shutdown_receiver.recv().await.unwrap();
}
