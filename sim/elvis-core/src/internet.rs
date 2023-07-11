use super::Machine;
use crate::{shutdown::ExitStatus, Shutdown};
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::Barrier, task::JoinSet};

pub async fn run_internet_with_timeout(machines: &[Machine], duration: Duration) -> ExitStatus {
    let future = run_internet(machines);
    let result = tokio::time::timeout(duration, future).await;
    match result {
        Ok(status) => status,
        Err(_) => ExitStatus::TimedOut,
    }
}

/// Runs the simulation with the given machines and networks
pub async fn run_internet(machines: &[Machine]) -> ExitStatus {
    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();

    let initialized = Arc::new(Barrier::new(total_protocols));

    // Spawn futures for every machine and then wait on them
    let mut handles = JoinSet::new();

    for machine in machines {
        let machine = machine.shallow_copy();
        let shutdown = shutdown.clone();
        let initialized = initialized.clone();

        let future = async move {
            machine.start(shutdown, initialized).await;
        };
        handles.spawn(future);
    }

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    let mut shutdown_receiver = shutdown.receiver();
    drop(shutdown);

    // wait for all starts to finish, or shutdown to occur, whichever happens first
    tokio::select! {
        _ = async {
            while let Some(result) = handles.join_next().await {
                result.expect("machines should be configured so internet can be run successfully");
            }
        } => (),
        result = shutdown_receiver.recv() => {
            match result {
                Err(_) => return ExitStatus::Exited,
                Ok(status) => return status
            }
        },
    }
    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let result = shutdown_receiver.recv().await;

    match result {
        Ok(status) => status,
        Err(_) => ExitStatus::Exited,
    }
}
