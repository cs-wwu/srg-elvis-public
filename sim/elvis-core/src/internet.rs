use super::Machine;
use crate::Protocol;
use crate::protocol::StartError;
use crate::{shutdown::ExitStatus, Shutdown};
use std::backtrace::Backtrace;
use std::panic;
use std::time::Duration;
use std::{process, sync::Arc};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::{sync::oneshot, task::JoinSet};

/// [`Protocol::start`] can send on a sender in order to
/// indicate that a machine is done initializing. 
pub type DoneSender = tokio::sync::oneshot::Sender<()>;

/// A struct used to coordinate a simulation.
/// Can be used to start waves of machines and wait for them to be initialized.
/// 
/// When a `Sim` is dropped, all of the protocols'
/// `start` methods (from [`initialize`](Sim::initialize)) are aborted.
#[derive(Default)]
pub struct Sim {
    // It may be possible to make this *not* a struct.

    // Used for debugging
    machines: Vec<Arc<Machine>>,

    /// Stores all the tasks for the running machines.
    tasks: JoinSet<Result<(), SimError>>,

    /// active shutdown object cloned and given to machines
    shutdown: Shutdown,
}

impl Sim {
    /// Creates a new Sim with no machines.
    pub fn new() -> Sim {
        Self::default()
    }

    /// Calls [`boot`](Protocol::boot) on each protocol of each machine. 
    /// Then spawns a task and calls [`start`](Protocol::start) for each protocol.
    /// 
    /// Once all protocols have sent messages through their `init_done` senders,
    /// this function completes.
    /// 
    /// (The machines `start`) methods will continue running in the background.
    /// 
    /// # Panics
    /// 
    /// If any of the machines return an error during boot, this method will panic.
    pub async fn init(&mut self, machines: &[Arc<Machine>]) {
        self.machines.extend_from_slice(machines);

        // Enable a custom panic hook to display a backtrace of the panic and then
        // forcibly exit the entire process.
        // If tests are being ran, this will stop future tests from running, as the
        // entire process exits, but that is appropriate for a panic    
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(make_exit_on_panic);

        // used to wait for machines to initialize
        let mut init_dones: FuturesUnordered<oneshot::Receiver<()>> = FuturesUnordered::new();

        // Boot every machine
        for (index, machine) in machines.iter().enumerate() {
            for protocol in machine.iter() {
                let result = protocol.boot(self.shutdown.clone(), machine.clone()).await;
                if let Err(err) = result {
                    panic!("Machine boot failed: {}", SimError {
                        index,
                        name: protocol.name(),
                        err,
                    });
                }
            }
        }

        // Start every protocol
        for (index, machine) in machines.iter().enumerate() {
            for protocol in machine.iter() {
                let (init_done, init_recv) = oneshot::channel();
                init_dones.push(init_recv);

                let fut = start_protocol(protocol, self.shutdown.clone(), init_done, machine.clone(), index);
                self.tasks.spawn(fut);
            }
        }


        // wait for all init_dones to complete
        while init_dones.next().await.is_some() {}
    }

    /// Calls `init` on all of the machines, one after another,
    /// so they are initialized in order.
    pub async fn init_order(&mut self, machines: impl IntoIterator<Item = Arc<Machine>>) {
        for machine in machines.into_iter() {
            self.init(&[machine]).await;
        }
    }

    /// Wait until a protocol shuts down the sim, or until all machines have dropped their
    /// `Shutdown`s.
    /// 
    /// Panics when a protocol panics or returns a `StartError`.
    pub async fn wait(&mut self) -> ExitStatus {
        tokio::select! {
            // panics when a protocol returns StartError or panics
            _ = async {
                while let Some(result) = self.tasks.join_next().await {
                    result.expect("A protocol panicked").expect("Protocols should be configured not to return StartError")
                }
            } => (),
            // gets status from the receiver
            status = self.shutdown.wait_for_shutdown() => return status,
        }
        
        self.shutdown.wait_for_shutdown().await
    }

    /// Gets an active [`Shutdown`] that can be used to shut down the sim.
    pub fn get_shutdown(&self) -> Shutdown {
        self.shutdown.clone()
    }

    /// Waits for the simulation to shut down.
    /// If it does not shut down after a given time,
    /// the simulation is shutdown with ExitStatus::TimedOut.
    pub async fn wait_with_timeout(&mut self, duration: Duration) -> ExitStatus {
        let result = tokio::time::timeout(duration, self.wait()).await;
        self.get_shutdown().shut_down_with_status(ExitStatus::TimedOut);

        match result {
            Ok(status) => status,
            Err(_) => ExitStatus::TimedOut,
        }
    }

    /// Waits for the simulation to shut down.
    /// If it does not shut down after a given time,
    /// the simulation is shut down with ExitStatus::TimedOut,
    /// then dropped after 1 second.
    pub async fn wait_force_timeout(mut self, duration: Duration) -> ExitStatus {
        let result = tokio::time::timeout(duration, self.wait()).await;
        self.get_shutdown().shut_down_with_status(ExitStatus::TimedOut);

        match result {
            Ok(status) => status,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                drop(self);
                ExitStatus::TimedOut
            },
        }
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        self.get_shutdown().shut_down();
    }
}

/// Calls protocol.start, but returns a SimError instead of a StartError.
/// Sorry it has 5 parameters
async fn start_protocol(protocol: Arc<dyn Protocol>, shutdown: Shutdown, init_done: DoneSender, machine: Arc<Machine>, index: usize) -> Result<(), SimError> {
    let result = protocol.start(shutdown, init_done, machine).await;
    match result {
        Ok(()) => Ok(()),
        Err(err) => Err(SimError {
            index,
            name: protocol.name(),
            err,
        })
    }
}

/// Sets a panic hook that exits the entire process.
fn make_exit_on_panic() {
    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        let backtrace = Backtrace::force_capture();
        eprintln!("Backtrace: {:#?}", backtrace);
        process::exit(1);
    }));
}

impl std::fmt::Debug for Sim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Sim ");
        let mut dl = f.debug_list();
        for machine in self.machines.iter() {
            dl.entry(&*machine);
        }
        dl.finish()
    }
}

#[derive(Clone, Debug)]
pub struct SimError {
    /// The index of the machine that returned the error.
    pub index: usize,
    /// The name of the protocol that returned the error.
    pub name: &'static str,
    /// The StartError from that machine.
    pub err: StartError,
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StartError for machine at index {}, protocol {}: {}", self.index, self.name, self.err)
    }
}

impl std::error::Error for SimError {}
