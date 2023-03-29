use super::Machine;
use crate::{Network, Shutdown};
use std::sync::Arc;
use tokio::sync::Barrier;

/// Runs the simulation with the given machines and networks
pub async fn run_internet(machines: Vec<Machine>, networks: Vec<Arc<Network>>) {
    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();
    let initialized = Arc::new(Barrier::new(total_protocols + networks.len()));

    for machine in machines {
        machine.start(shutdown.clone(), initialized.clone());
    }

    for network in networks {
        network.start(shutdown.clone(), initialized.clone());
    }

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    let mut shutdown_receiver = shutdown.receiver();
    drop(shutdown);

    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let _ = shutdown_receiver.recv().await;
}
