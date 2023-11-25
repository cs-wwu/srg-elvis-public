use crate::{Protocol, Shutdown};
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::{Arc, OnceLock}
};
use tokio::{sync::Barrier, task::JoinSet};

/// A tap's PCI slot index
pub type PciSlot = u32;

type ArcAny = Arc<dyn Any + Send + Sync + 'static>;
type AnyMap = FxHashMap<TypeId, (ArcAny, Arc<dyn Protocol>)>;

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines
/// communicate through [`Network`](super::Network)s. Each machine contains a
/// set of [`Protocol`]s that it manages. The protocols may be
/// networking protocols or user programs.
#[derive(Default)]
pub struct Machine {
    protocols: AnyMap,
    pub name: OnceLock<String>
}

impl Machine {
    /// Creates a new machine with no protocols.
    /// Protocols can be added using the [`crate::Machine::with`] method.
    ///
    /// We recommend using the [`new_machine`] macro to create a machine instead.
    pub fn new() -> Machine {
        Self {
            protocols: AnyMap::default(),
            name: OnceLock::new(),
        }
    }

    /// Tells the machine time to start its protocols and begin participating in the simulation.
    ///
    /// Calls [`start()`](super::Protocol::start) on all its protocols, then waits for them to finish.
    /// Panics if any of them return an error.
    pub(crate) async fn start(self: Arc<Self>, shutdown: Shutdown, initialized: Arc<Barrier>) {
        let mut handles = JoinSet::new();

        // Spawn tasks to start each protocol
        for protocol in self.iter() {
            let shutdown_clone = shutdown.clone();
            let initialized_clone = initialized.clone();
            let self_clone = self.clone();
            let fut = async move {
                protocol
                    .start(shutdown_clone, initialized_clone, self_clone)
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
        let protocol = Arc::new(protocol);
        self.protocols.insert(
            TypeId::of::<T>(),
            (protocol.clone() as ArcAny, protocol as Arc<dyn Protocol>),
        );
        self
    }

    /// Returns the protocol of type `T` from this machine, or returns `None` if it does not exist.
    pub fn protocol<T>(&self) -> Option<Arc<T>>
    where
        T: Protocol,
    {
        self.protocols
            .get(&TypeId::of::<T>())
            .map(|t| t.0.clone().downcast().unwrap())
    }

    /// Returns the protocol with type ID `id` from this machine, or returns `None` if it does not exist.
    pub fn get(&self, id: TypeId) -> Option<Arc<dyn Protocol>> {
        self.protocols.get(&id).map(|t| t.1.clone())
    }

    /// Creates an iterator over this machine's protocols.
    pub fn iter(&self) -> impl Iterator<Item = Arc<dyn Protocol>> + '_ {
        self.protocols.values().map(|t| t.1.clone())
    }

    /// Places this machine inside of an [`Arc`].
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
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
