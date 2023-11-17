use super::Machine;
use crate::{shutdown::ExitStatus, Shutdown};
use std::backtrace::Backtrace;
use std::panic;
use std::time::Duration;
use std::{process, sync::Arc};
use tokio::{sync::Barrier, task::JoinSet, time::sleep};

/// Runs the simulation with the given machines
/// This function will call run_internet() with the provided timeout,
/// then forcibly shut down the simulation if it fails to do so itself
/// one second after the call to shutdown.shut_down()
pub async fn run_internet_with_timeout(
    machines: &[Arc<Machine>],
    duration: Duration,
) -> ExitStatus {
    let future = run_internet(machines, Some(duration));
    let result = tokio::time::timeout(duration + Duration::from_secs(1), future).await;
    match result {
        Ok(status) => status,
        Err(_) => ExitStatus::TimedOut,
    }
}

/// Runs the simulation with the given machines
/// `timeout` is an optional field, if Some() is provided, this function
/// will call shutdown.shut_down() after the given duration.
pub async fn run_internet(machines: &[Arc<Machine>], timeout: Option<Duration>) -> ExitStatus {
    // Enable a custom panic hook to display a backtrace of the panic and then
    // forcibly exit the entire process.
    // If tests are being ran, this will stop future tests from running, as the
    // entire process exits, but that is appropriate for a panic
    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        let backtrace = Backtrace::force_capture();
        eprintln!("Backtrace: {:#?}", backtrace);
        process::exit(1);
    }));

    let shutdown = Shutdown::new();
    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();

    let initialized = Arc::new(Barrier::new(total_protocols));

    // Spawn futures for every machine and then wait on them
    let mut handles = JoinSet::new();

    for machine in machines {
        let machine = machine.clone();
        let shutdown = shutdown.clone();
        let initialized = initialized.clone();
        handles.spawn(machine.start(shutdown, initialized));
    }

    if let Some(duration) = timeout {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            sleep(duration).await;
            println!("Waking up and shutting down");
            shutdown.shut_down();
        });
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
