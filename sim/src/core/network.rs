use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::protocols::tap::NetworkInfo;

use super::{
    control::{Primitive, PrimitiveError},
    message::Message,
    Machine, MachineId,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

/// A maximum transmission unit
pub type Mtu = u32;

pub type NetworkId = usize;

type Pending = HashMap<usize, Vec<Message>>;

/// A link-level connection between [`Machine`](super::Machine)s.
///
/// A network facilitates connecting multiple machines together and allowing
/// them to exchange [`Message`]s. Roughly, it models an simplified Ethernet
/// network with broadcast and MAC-based message delivery.
pub struct Network {
    mtu: Mtu,
    receivers: Vec<Receiver<Postmarked>>,
    senders: HashMap<MachineId, Sender<Delivery>>,
    id: NetworkId,
}

impl Network {
    /// Create a new network with the given `mtu` and list of networked
    /// [`Machine`](super::Machine)s.
    pub fn new(id: NetworkId, mtu: Mtu) -> Self {
        Self {
            receivers: Default::default(),
            senders: Default::default(),
            mtu,
            id,
        }
    }

    pub fn id(&self) -> NetworkId {
        self.id
    }

    pub async fn start(&mut self) {
        let mut futures: FuturesUnordered<_> = self
            .receivers
            .iter_mut()
            .map(|receiver| receiver.recv())
            .collect();
        while let Some(Some(next)) = futures.next().await {
            let delivery = Delivery {
                message: next.message,
                network: self.id,
            };
            match next.address {
                PhysicalAddress::Recipient(mac) => match self.senders.entry(mac) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().send(delivery).await;
                    }
                    Entry::Vacant(entry) => panic!("No machine found with that ID"),
                },
                PhysicalAddress::Broadcast => {
                    for sender in self.senders.values_mut() {
                        sender.send(delivery.clone()).await;
                    }
                }
            }
        }
    }

    pub fn attach(&mut self, machine: &mut Machine) {
        let (machine_sender, network_receiver) = mpsc::channel(16);
        let (network_sender, machine_receiver) = mpsc::channel(16);
        match self.senders.entry(machine.id()) {
            Entry::Occupied(entry) => panic!("Attaching the same machine to the network twice"),
            Entry::Vacant(entry) => {
                entry.insert(network_sender);
            }
        }
        self.receivers.push(network_receiver);
        let info = NetworkInfo {
            mtu: self.mtu,
            network_id: self.id.into(),
            sender: machine_sender,
            receiver: machine_receiver,
        };
        machine.attach(info);
    }

    /// The network's maximum transmission unit.
    pub fn mtu(&self) -> Mtu {
        self.mtu
    }

    /// The list of connected machines.
    pub fn connected_machines(&self) -> impl Iterator<Item = &MachineId> {
        self.senders.keys()
    }

    /// Send a `message` to the machine or machines identified by `address`.
    pub async fn send(&mut self, address: PhysicalAddress, message: Message) {
        // TODO(hardint): Check that the message is shorter than MTU
    }
}

/// Describes to whom to send a [`Message`] across a [`Network`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalAddress {
    /// Send the message to a particular machine on the network
    Recipient(MachineId),
    /// Send the message to all machines on the network
    Broadcast,
}

#[derive(Clone)]
pub struct Postmarked {
    pub message: Message,
    pub address: PhysicalAddress,
}

#[derive(Clone)]
pub struct Delivery {
    pub message: Message,
    pub network: NetworkId,
}

impl From<NetworkId> for Primitive {
    fn from(id: NetworkId) -> Self {
        id.into()
    }
}

impl TryFrom<Primitive> for NetworkId {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.try_into()
    }
}
