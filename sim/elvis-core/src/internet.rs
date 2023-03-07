use super::Machine;
use crate::{Network, Shutdown};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;
use tokio_metrics::{TaskMetrics, TaskMonitor};

/// Runs the simulation with the given machines and networks
pub async fn run_internet(machines: Vec<Machine>, networks: Vec<Arc<Network>>) {
    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();
    let initialized = Arc::new(Barrier::new(total_protocols + networks.len()));

    let pci_monitor = TaskMonitor::new();
    for machine in machines {
        machine.start(shutdown.clone(), initialized.clone(), pci_monitor.clone());
    }

    let network_monitor = TaskMonitor::new();
    for network in networks {
        network.start(
            shutdown.clone(),
            initialized.clone(),
            network_monitor.clone(),
        );
    }

    const METRICS_FREQUENCY: Duration = Duration::from_secs(1);
    tokio::spawn(async move {
        for (pci, network) in pci_monitor.intervals().zip(network_monitor.intervals()) {
            print!("PCI = ");
            print_metrics(pci);
            print!("Network = ");
            print_metrics(network);
            println!();
            tokio::time::sleep(METRICS_FREQUENCY).await;
        }
    });

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    let mut shutdown_receiver = shutdown.receiver();
    drop(shutdown);

    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let _ = shutdown_receiver.recv().await;
}

fn print_metrics(metrics: TaskMetrics) {
    println!(
        "{{
    idle      = {:?}, {:?}
    scheduled = {:?}, {:?}
    poll      = {:?}, {:?}
}}",
        metrics.total_idle_duration,
        metrics.mean_idle_duration(),
        metrics.total_scheduled_duration,
        metrics.mean_scheduled_duration(),
        metrics.total_poll_duration,
        metrics.mean_poll_duration(),
    )
}
