use crate::{protocol::SharedProtocol, Protocol, Shutdown};
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tokio::sync::Barrier;

/// A tap's PCI slot index
pub(crate) type PciSlot = u32;

type ArcAny = Arc<dyn Any + Send + Sync + 'static>;
type AnyMap = FxHashMap<TypeId, (ArcAny, SharedProtocol)>;

#[derive(Default)]
pub struct ProtocolMapBuilder {
    inner: AnyMap,
}

impl ProtocolMapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with<T>(mut self, protocol: T) -> Self
    where
        T: Protocol + Send + Sync + 'static,
    {
        let protocol = Arc::new(protocol);
        self.inner.insert(
            TypeId::of::<T>(),
            (protocol.clone() as ArcAny, protocol as SharedProtocol),
        );
        self
    }

    pub fn build(self) -> ProtocolMap {
        ProtocolMap {
            inner: Arc::new(self.inner),
        }
    }
}

/// A mapping of protocol IDs to protocols
#[derive(Clone)]
pub struct ProtocolMap {
    inner: Arc<AnyMap>,
}

impl ProtocolMap {
    pub fn protocol<T>(&self) -> Option<Arc<T>>
    where
        T: Protocol + Send + Sync + 'static,
    {
        self.inner
            .get(&TypeId::of::<T>())
            .map(|t| t.0.clone().downcast().unwrap())
    }

    pub fn get(&self, id: TypeId) -> Option<SharedProtocol> {
        self.inner.get(&id).map(|t| t.1.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = SharedProtocol> + '_ {
        self.inner.values().map(|t| t.1.clone())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines
/// communicate through [`Network`](super::Network)s. Each machine contains a
/// set of [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub struct Machine {
    protocols: ProtocolMap,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(protocols: ProtocolMap) -> Machine {
        Self { protocols }
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub(crate) fn start(&self, shutdown: Shutdown, initialized: Arc<Barrier>) {
        for protocol in self.protocols.iter() {
            protocol
                .start(
                    shutdown.clone(),
                    initialized.clone(),
                    self.protocols.clone(),
                )
                .expect("A protocol failed to start")
        }
    }

    /// The number of protocols in the machine.
    pub fn protocol_count(&self) -> usize {
        self.protocols.len()
    }

    pub fn into_inner(self) -> ProtocolMap {
        self.protocols
    }
}

/// Creates a [`Machine`] with the protocols given.
///
/// # Example
///
/// ```
/// use elvis_core::{
///     protocols::*,
///     run_internet,
///     machine::*,
/// };
///
/// let machines = [
///     new_machine![
///         Ipv4::new(std::iter::empty().collect()),
///         Pci::new([]),
///     ],
///     new_machine![
///         Udp::new(),
///         Ipv4::new(std::iter::empty().collect()),
///         Pci::new([]),
///     ],
///     new_machine![],
/// ];
///
/// run_internet(&machines);
/// ```
#[macro_export]
macro_rules! new_machine {
    ( $($x:expr),* $(,)? ) => {
        {
            let mut pmb = ProtocolMapBuilder::new()
            $(
                .with($x)
            )*;
            Machine::new(pmb.build())
        }
    };
}
pub use new_machine;
