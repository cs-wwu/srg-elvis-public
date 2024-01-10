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
    // IMPORTANT: we must create the shutdown receiver before starting the machines
    // so it can receive shutdown messages!
    let mut shutdown_receiver = shutdown.clone().receiver();

    let total_protocols: usize = machines
        .iter()
        .map(|machine| machine.protocol_count())
        .sum();

    let initialized = Arc::new(Barrier::new(total_protocols));

    // Spawn futures for every machine and then wait on them
    let mut handles = JoinSet::new();

    let mut counter = 0;

    for machine in machines {
        let machine = machine.clone();
        machine.name.set(counter.to_string()).unwrap();
        counter += 1;
        let shutdown = shutdown.clone();
        let initialized = initialized.clone();
        handles.spawn(machine.start(shutdown, initialized));
    }

    if let Some(duration) = timeout {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            sleep(duration).await;
            println!("Waking up and shutting down");
            shutdown.shut_down_with_status(ExitStatus::TimedOut);
        });
    }

    // We drop our shutdown first because otherwise, the recv() sleeps forever
    drop(shutdown);

    // wait for all starts to finish, or shutdown to occur, whichever happens first
    tokio::select! {
        _ = async {
            while let Some(result) = handles.join_next().await {
                result.expect("machines should be configured so internet can be run successfully");
            }
        } => (),
        result = get_status(&mut shutdown_receiver) => return result,
    }
    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    get_status(&mut shutdown_receiver).await
}

/// Gets the shutdown status from a broadcast receiver.
/// Returns ExitStatus::Exited if the channel is closed.
async fn get_status(
    shutdown_receiver: &mut tokio::sync::broadcast::Receiver<ExitStatus>,
) -> ExitStatus {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match shutdown_receiver.recv().await {
            Ok(status) => return status,
            Err(RecvError::Closed) => return ExitStatus::Exited,
            Err(RecvError::Lagged(_)) => continue,
        }
    }
}
