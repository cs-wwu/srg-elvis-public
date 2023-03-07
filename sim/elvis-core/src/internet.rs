use super::Machine;
use crate::{network::NetworkMonitors, Network, Shutdown};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tokio::sync::Barrier;
use tokio_metrics::{Instrumented, TaskMetrics, TaskMonitor};

/// Runs the simulation with the given machines and networks
pub async fn run_internet(
    machines: Vec<Machine>,
    networks: Vec<Arc<Network>>,
    mut monitors: Vec<MonitorInfo>,
) {
    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();
    let initialized = Arc::new(Barrier::new(total_protocols + networks.len()));

    for machine in machines {
        machine.start(shutdown.clone(), initialized.clone());
    }

    let network_monitors = NetworkMonitors::new();
    for network in networks {
        network.start(
            shutdown.clone(),
            initialized.clone(),
            network_monitors.clone(),
        );
    }
    monitors.extend(network_monitors);

    const METRICS_FREQUENCY: Duration = Duration::from_secs(1);
    tokio::spawn(async move {
        let mut intervals: Vec<_> = monitors
            .into_iter()
            .map(|info| (info.monitor.intervals(), info.name))
            .collect();
        loop {
            tokio::time::sleep(METRICS_FREQUENCY).await;
            for (interval, name) in intervals.iter_mut() {
                print_interval(name, interval.next().unwrap());
            }
            println!("\n");
        }
    });

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    let mut shutdown_receiver = shutdown.receiver();
    drop(shutdown);

    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let _ = shutdown_receiver.recv().await;
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: &'static str,
    pub monitor: TaskMonitor,
}

impl MonitorInfo {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            monitor: TaskMonitor::new(),
        }
    }

    pub fn instrument<F>(&self, task: F) -> Instrumented<F> {
        self.monitor.instrument(task)
    }
}

fn print_interval(name: &'static str, interval: TaskMetrics) {
    println!(
        "{} = {{
    idle      = {:06?}ms, {:?}
    scheduled = {:06?}ms, {:?}
    poll      = {:06?}ms, {:?}
}}",
        name,
        interval.total_idle_duration.as_millis(),
        interval.total_idled_count,
        interval.total_scheduled_duration.as_millis(),
        interval.total_scheduled_count,
        interval.total_poll_duration.as_millis(),
        interval.total_poll_count,
    )
}
