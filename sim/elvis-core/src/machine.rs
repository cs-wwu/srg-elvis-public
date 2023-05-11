use crate::{
    protocol::SharedProtocol,
    protocols::{Ipv4, Pci, Tcp, Udp},
    Id, Shutdown,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::Barrier;

/// A tap's PCI slot index
pub(crate) type PciSlot = u32;

#[derive(Default)]
pub struct ProtocolMapBuilder {
    pci: Option<Pci>,
    ipv4: Option<Ipv4>,
    udp: Option<Udp>,
    tcp: Option<Tcp>,
    other: FxHashMap<Id, SharedProtocol>,
}

impl ProtocolMapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pci(mut self, pci: Pci) -> Self {
        self.pci = Some(pci);
        self
    }

    pub fn ipv4(mut self, ipv4: Ipv4) -> Self {
        self.ipv4 = Some(ipv4);
        self
    }

    pub fn udp(mut self, udp: Udp) -> Self {
        self.udp = Some(udp);
        self
    }

    pub fn tcp(mut self, tcp: Tcp) -> Self {
        self.tcp = Some(tcp);
        self
    }

    pub fn other(mut self, other: SharedProtocol) -> Self {
        self.other.insert(other.id(), other);
        self
    }

    pub fn build(self) -> ProtocolMap {
        ProtocolMap {
            pci: self.pci.map(|pci| Arc::new(pci)),
            ipv4: self.ipv4.map(|ipv4| Arc::new(ipv4)),
            udp: self.udp.map(|udp| Arc::new(udp)),
            tcp: self.tcp.map(|tcp| Arc::new(tcp)),
            other: Arc::new(self.other),
        }
    }
}

/// A mapping of protocol IDs to protocols
#[derive(Clone)]
pub struct ProtocolMap {
    pci: Option<Arc<Pci>>,
    ipv4: Option<Arc<Ipv4>>,
    udp: Option<Arc<Udp>>,
    tcp: Option<Arc<Tcp>>,
    other: Arc<FxHashMap<Id, SharedProtocol>>,
}

impl ProtocolMap {
    pub fn protocol(&self, id: Id) -> Option<SharedProtocol> {
        match id {
            Pci::ID => self.pci.as_ref().map(|p| p.clone() as SharedProtocol),
            Udp::ID => self.udp.as_ref().map(|p| p.clone() as SharedProtocol),
            Tcp::ID => self.tcp.as_ref().map(|p| p.clone() as SharedProtocol),
            Ipv4::ID => self.ipv4.as_ref().map(|p| p.clone() as SharedProtocol),
            _ => self.other.get(&id).cloned(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = SharedProtocol> + '_ {
        [
            self.pci.clone().map(|p| p as SharedProtocol),
            self.ipv4.clone().map(|p| p as SharedProtocol),
            self.udp.clone().map(|p| p as SharedProtocol),
            self.tcp.clone().map(|p| p as SharedProtocol),
        ]
        .into_iter()
        .filter_map(|p| p)
        .chain(self.other.values().cloned())
    }

    pub fn len(&self) -> usize {
        self.iter().count()
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
    pub(crate) fn start(self, shutdown: Shutdown, initialized: Arc<Barrier>) {
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
}
