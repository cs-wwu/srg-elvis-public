use super::Machine;
use crate::Protocol;
use crate::protocol::StartError;
use crate::{shutdown::ExitStatus, Shutdown};
use std::backtrace::Backtrace;
use std::panic;
use std::time::Duration;
use std::{process, sync::Arc};
use futures::Future;
use tokio::{sync::broadcast, task::JoinSet};

/// A struct used to coordinate a simulation.
/// Can be used to start waves of machines and wait for them to be initialized.
/// 
/// When a `Sim` is dropped, all of the protocols'
/// `start` methods (from [`initialize`](Sim::initialize)) are aborted.
/// 
/// # Examples
/// 
/// ```
/// 
/// use elvis_core::protocols::Pci;
/// use elvis_core::internet::Sim;
/// use elvis_core::shutdown::ExitStatus;
/// 
/// #[tokio::main]
/// # fn main() {
/// // Create a machine.
/// // (Usually a machine will have more protocols than just Pci!)
/// let machine = elvis_core::new_machine_arc![
///     Pci::new([]),
/// ];
/// 
/// let sim = Sim::new();
/// 
/// // Initialize the machine.
/// sim.init(machine).await;
/// 
/// // Wait for simulation to finish, with a timeout..
/// let exit_status = sim.wait_with_timeout(Duration::from_millis(1)).await;
/// assert_eq!(exit_status, ExitStatus::TimedOut);
/// # }
/// ```
#[derive(Default)]
pub struct Sim {
    /// Used for debugging
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

    /// Spawns a task to call [`boot`](Protocol::boot) on each protocol of each machine.
    /// If all boots are successful, calls [`start`](Protocol::start) for each protocol.
    /// 
    /// This function returns a future.
    /// Once all protocols have sent messages through their `init_done` senders,
    /// the future completes.
    /// 
    /// (The machine's `start`) methods will continue running in the background.
    /// 
    /// See [Sim] for an example.
    pub fn init(&mut self, machine: Arc<Machine>) -> impl Future<Output = ()> {
        self.machines.push(machine);

        // Enable a custom panic hook to display a backtrace of the panic and then
        // forcibly exit the entire process.
        // If tests are being ran, this will stop future tests from running, as the
        // entire process exits, but that is appropriate for a panic    
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(make_exit_on_panic);

        let (send, mut recv) = broadcast::channel(1);

        for (index, machine) in self.machines.iter().enumerate() {
            self.tasks.spawn(start_machine(machine.clone(), self.shutdown.clone(), send.clone(), index));
        }


        use broadcast::error::RecvError;
        async move {
            // The receiver should receive if and only if all the senders are dropped.
            assert_eq!(recv.recv().await, Err(RecvError::Closed));
        }
    }

    /// Calls [`Sim::init`] on all of the machines, one after another,
    /// so they are initialized in order.
    pub async fn init_order(&mut self, machines: &[Arc<Machine>]) {
        for machine in machines.into_iter().cloned() {
            self.init(machine).await;
        }
    }

    /// Calls [`Sim::init`] on all the machines in parallel.
    pub async fn init_parallel(&mut self, machines: &[Arc<Machine>]) {
        let mut futs = Vec::new();
        for machine in machines.into_iter().cloned() {
            let fut = self.init(machine);
            futs.push(fut);
        }
        futures::future::join_all(futs).await;
    }

    /// Wait until a protocol shuts down the sim, or until all machines have dropped their
    /// `Shutdown`s.
    /// 
    /// Panics when a protocol panics or returns a `StartError`.
    pub async fn wait(&mut self) -> ExitStatus {
        tokio::select! {
            _ = async {
                while let Some(result) = self.tasks.join_next().await {
                    result.expect("A protocol panicked").expect("Protocols should be configured not to return StartError")
                }
            } => (),
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

/// A struct that the client can use to notify a simulation that it is initialized.
/// Currently, this implemented as a wrapper around a Tokio [`oneshot::Sender`].
#[derive(Debug)]
pub struct DoneSender {
    /// The index of the machine in the simulation.
    index: usize,
    /// The name of the protocol using this DoneSender.
    name: &'static str,
    /// The sim knows that all protocols are done initializing when
    /// all these senders are dropped
    inner: broadcast::Sender<()>,
}

impl DoneSender {
    /// Call this method to notify the other end that your protocol is finished
    /// initializing.
    /// (See [Sim::init].)
    pub fn done(self) {
        tracing::info!("Machine {}, protocol {} finished initializing", self.index, self.name);
        drop(self)
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

/// Calls [`start`] then [`boot`] on each protocol of the given machine.
/// Returns StartError if either of those returned that.
async fn start_machine(machine: Arc<Machine>, shut: Shutdown, send: broadcast::Sender<()>, index: usize) -> Result<(), SimError> {
    // boot every protocol
    for protocol in machine.iter() {
        let result = protocol.boot(shut.clone(), machine.clone()).await;
        if let Err(err) = result {
            return Err(SimError {
                index,
                name: protocol.name(),
                err,
            });
        }
    }

    // start every protocol
    let mut tasks: JoinSet<Result<(), SimError>> = JoinSet::new();
    for protocol in machine.iter() {
        let ds = DoneSender {
            index,
            name: protocol.name(),
            inner: send.clone(),
        };
        let fut = start_protocol(protocol, shut.clone(), ds, machine.clone(), index);
        tasks.spawn(fut);
    }

    // wait for every protocol to be done
    while let Some(result) = tasks.join_next().await {
        // It's a Result<Result<(), SimError>, JoinError>
        // So we handle the JoinError first.
        result.expect("Protocol start tasks should not be cancelled or dropped this early")?;
    }
    Ok(())
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
