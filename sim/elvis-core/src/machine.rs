use crate::{protocol::StartError, Protocol, Shutdown};
use futures::future::BoxFuture;
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tokio::{sync::Barrier, task::JoinSet};

/// A tap's PCI slot index
pub type PciSlot = u32;

type ArcAny = Arc<dyn Any + Send + Sync + 'static>;
type AnyMap = FxHashMap<TypeId, ProtocolRefs>;

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines
/// communicate through [`Network`](super::Network)s. Each machine contains a
/// set of [`Protocol`]s that it manages. The protocols may be
/// networking protocols or user programs.
#[derive(Default)]
pub struct Machine {
    protocols: AnyMap,
}

/// This struct is used to hold 3 different kinds of refs to the same
/// Protocol, because Rust doesn't support trait upcasting :(
#[derive(Clone)]
struct ProtocolRefs {
    /// Used so we can downcast the protocol
    any: ArcAny,
    /// Used so we can iterate over the protocol
    protocol: Arc<dyn Protocol>,
    /// Used so we can call .start on the protocol
    dynprotocolstart: Arc<dyn DynProtocolStart>,
}

impl Machine {
    /// Creates a new machine with no protocols.
    /// Protocols can be added using the [`crate::Machine::with`] method.
    ///
    /// We recommend using the [`new_machine`] macro to create a machine instead.
    pub fn new() -> Machine {
        Self {
            protocols: AnyMap::default(),
        }
    }

    /// Tells the machine time to start its protocols and begin participating in the simulation.
    ///
    /// Calls [`start()`](super::Protocol::start) on all its protocols, then waits for them to finish.
    /// Panics if any of them return an error.
    pub(crate) async fn start(self: Arc<Self>, shutdown: Shutdown, initialized: Arc<Barrier>) {
        let mut handles = JoinSet::new();

        // Spawn tasks to start each protocol
        for protocol in self.dyn_start_iter() {
            let shutdown_clone = shutdown.clone();
            let initialized_clone = initialized.clone();
            let self_clone = self.clone();
            let fut = async move {
                protocol
                    .dyn_start(shutdown_clone, initialized_clone, self_clone)
                    .await
            };
            handles.spawn(fut);
        }

        // wait for all starts to finish
        while let Some(result) = handles.join_next().await {
            result
                .expect("start method should not panic!")
                .expect("machines should be configured to start successfully");
        }
    }

    /// Returns the number of protocols in this machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }

    /// Adds `protocol` to this `Machine`, then returns itself.
    pub fn with<T>(mut self, protocol: T) -> Self
    where
        T: Protocol + Send + Sync + 'static,
    {
        // make ProtocolRefs struct
        let arc = Arc::new(protocol);
        let protocol = ProtocolRefs {
            any: arc.clone(),
            protocol: arc.clone(),
            dynprotocolstart: arc.clone(),
        };
        self.protocols.insert(TypeId::of::<T>(), protocol);
        self
    }

    /// Returns the protocol of type `T` from this machine, or returns `None` if it does not exist.
    pub fn protocol<T>(&self) -> Option<Arc<T>>
    where
        T: Protocol,
    {
        self.protocols
            .get(&TypeId::of::<T>())
            .map(|t| t.any.clone().downcast().unwrap())
    }

    /// Returns the protocol with type ID `id` from this machine, or returns `None` if it does not exist.
    pub fn get(&self, id: TypeId) -> Option<Arc<dyn Protocol>> {
        self.protocols.get(&id).map(|t| t.protocol.clone())
    }

    /// Creates an iterator over this machine's protocols.
    pub fn iter(&self) -> impl Iterator<Item = Arc<dyn Protocol>> + '_ {
        self.protocols.values().map(|t| t.protocol.clone())
    }

    /// Creates an iterator over this machine's protocols
    /// (with the DynProtocolStart trait).
    fn dyn_start_iter(&self) -> impl Iterator<Item = Arc<dyn DynProtocolStart>> + '_ {
        self.protocols.values().map(|t| t.dynprotocolstart.clone())
    }

    /// Places this machine inside of an [`Arc`].
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

/// The [`Protocol::start`] method only works on concrete protocols.
/// This extension trait adds a start method which can be used on
/// dyn Protocols.
trait DynProtocolStart: Protocol {
    fn dyn_start(
        self: Arc<Self>,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> BoxFuture<'static, Result<(), StartError>>;
}

impl<P: Protocol> DynProtocolStart for P {
    fn dyn_start(
        self: Arc<Self>,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> BoxFuture<'static, Result<(), StartError>> {
        let fut = async move { self.start(shutdown, initialized, machine).await };
        Box::pin(fut)
    }
}

/// Creates a [`Machine`] with the protocols given.
///
/// ```
/// # use elvis_core::{new_machine, protocols::Pci};
/// let _ = new_machine![Pci::new([])];
/// ```
///
/// is the same as:
///
/// ```
/// # use elvis_core::{Machine, protocols::Pci};
/// let _ = Machine::new()
///             .with(Pci::new([]));
/// ```
///
/// # Example
///
/// ```
/// use elvis_core::{
///     protocols::*,
///     run_internet,
///     machine::*,
///     IpTable
/// };
///
/// let machines = [
///     new_machine![
///         Ipv4::new(IpTable::new()),
///         Pci::new([]),
///     ].arc(),
///     new_machine![
///         Udp::new(),
///         Ipv4::new(IpTable::new()),
///         Pci::new([]),
///     ].arc(),
///     new_machine![].arc(),
/// ];
///
/// run_internet(&machines, None);
/// ```
#[macro_export]
macro_rules! new_machine {
    ( $($x:expr),* $(,)? ) => {
        {
            $crate::Machine::new()
            $(
                .with($x)
            )*
        }
    };
}
pub use new_machine;

/// A version of the [`new_machine`] macro which puts the resulting machine in an [`Arc`].
///
/// ```
/// # use elvis_core::{new_machine_arc, protocols::Pci};
/// let _ = new_machine_arc![Pci::new([])];
/// ```
///
/// Is the same as:
///
/// ```
/// # use elvis_core::{new_machine, protocols::Pci};
/// let _ = new_machine![Pci::new([])].arc();
/// ```
///
/// Is the same as:
///
/// ```
/// # use elvis_core::{Machine, protocols::Pci};
/// let _ = Machine::new()
///             .with(Pci::new([]))
///             .arc();
/// ```
#[macro_export]
macro_rules! new_machine_arc {
    ( $($x:expr),* $(,)? ) => {
        {
            $crate::new_machine![
                $($x, )*
            ].arc()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        protocols::{Ipv4, Pci},
        IpTable,
    };
    #[test]
    fn test() {
        let _machine = new_machine![Ipv4::new(IpTable::new()), Pci::new([]),];
    }
}
