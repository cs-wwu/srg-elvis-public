use crate::{Protocol, Shutdown};
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tokio::{sync::Barrier, task::JoinSet};

/// A tap's PCI slot index
pub(crate) type PciSlot = u32;

type ArcAny = Arc<dyn Any + Send + Sync + 'static>;
type AnyMap = FxHashMap<TypeId, (ArcAny, Arc<dyn Protocol>)>;

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
            (protocol.clone() as ArcAny, protocol as Arc<dyn Protocol>),
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
        T: Protocol,
    {
        self.inner
            .get(&TypeId::of::<T>())
            .map(|t| t.0.clone().downcast().unwrap())
    }

    pub fn get(&self, id: TypeId) -> Option<Arc<dyn Protocol>> {
        self.inner.get(&id).map(|t| t.1.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = Arc<dyn Protocol>> + '_ {
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
    pub network_api: NetworkAPI,
    pub protocols: ProtocolMap,
}

impl Machine {
    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new(protocols: ProtocolMap) -> Machine {
        Self { protocols }
    }


    /// Creates a new machine containing the given `protocols`. Returns the
    /// machine and a channel which can be used to send messages to the machine.
    pub fn new_auth_dns(
        auth_ip: Ipv4Address,
        network: Arc<Network>,
        ip_table: Recipients,
    ) -> Machine {
        let socket_api = Sockets::new(Some(auth_ip)).shared();
        Machine::new([
            socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            Dns::new(DnsType::AUT, auth_ip).shared(),
            // TODO(zachd9757): DnsServer app for doing auth server stuff (wait/listen/etc.)
            // DnsServer::new();
        ])
    }

    /// Tells the machine time to [`start()`](super::Protocol::start) its
    /// protocols and begin participating in the simulation.
    pub(crate) async fn start(&self, shutdown: Shutdown, initialized: Arc<Barrier>) {
        let mut handles = JoinSet::new();

        // Spawn tasks to start each protocol
        for protocol in self.protocols.iter() {
            let shutdown_clone = shutdown.clone();
            let initialized_clone = initialized.clone();
            let protocols_clone = self.protocols.clone();
            let future = async move {
                protocol
                    .start(shutdown_clone, initialized_clone, protocols_clone)
                    .await
            };

            handles.spawn(future);
        }

        // wait for all starts to finish
        while let Some(result) = handles.join_next().await {
            result
                .expect("start method should not panic!")
                .expect("machines should be configured to start successfully");
        }
    }

    /// Creates a copy of itself that points to the same protocols.
    pub(crate) fn shallow_copy(&self) -> Self {
        Self::new(self.protocols.clone())
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

            let pmb = $crate::machine::ProtocolMapBuilder::new()
            $(
                .with($x)
            )*;
            $crate::Machine::new(pmb.build())
        }
    };
}
pub use new_machine;

#[cfg(test)]
mod tests {
    use crate::protocols::{Ipv4, Pci};
    #[test]
    fn test() {
        let _machine = new_machine![Ipv4::new(std::iter::empty().collect()), Pci::new([]),];
    }
}
