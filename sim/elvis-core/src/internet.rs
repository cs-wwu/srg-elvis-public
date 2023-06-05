use super::Machine;
use crate::Shutdown;
use std::sync::Arc;
use tokio::sync::Barrier;

/// Runs the simulation with the given machines and networks
pub async fn run_internet(machines: impl IntoIterator<Item = Machine>) {
    let machines: Vec<Machine> = machines.into_iter().collect();

    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();
    let initialized = Arc::new(Barrier::new(total_protocols));

    // Spawn futures for every machine and then wait on them
    let mut handles = Vec::new();
    for machine in machines {
        let shutdown_clone = shutdown.clone();
        let initialized_clone = initialized.clone();
        let future = async move {
            machine.start(shutdown_clone, initialized_clone).await;
        };
        let future = tokio::spawn(future);
        handles.push(future);
    }
    futures::future::try_join_all(handles)
        .await
        .expect("machines should be configured not to error");

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    let mut shutdown_receiver = shutdown.receiver();
    drop(shutdown);

    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let _ = shutdown_receiver.recv().await;
}
