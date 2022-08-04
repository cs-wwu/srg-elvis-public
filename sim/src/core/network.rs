use futures::stream::{FuturesUnordered, StreamExt};
use std::mem;
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

    pub fn start(&mut self) {
        let mut receivers = mem::replace(&mut self.receivers, Default::default());
        let id = self.id;
        let mut senders = mem::replace(&mut self.senders, Default::default());
        tokio::spawn(async move {
            let mut futures: FuturesUnordered<_> = receivers
                .iter_mut()
                .map(|receiver| receiver.recv())
                .collect();
            while let Some(Some(next)) = futures.next().await {
                let delivery = Delivery {
                    message: next.message,
                    network: id,
                };
                match next.address {
                    PhysicalAddress::Recipient(mac) => match senders.entry(mac) {
                        Entry::Occupied(mut entry) => {
                            entry.get_mut().send(delivery).await.unwrap();
                        }
                        Entry::Vacant(_) => panic!("No machine found with that ID"),
                    },
                    PhysicalAddress::Broadcast => {
                        for sender in senders.values_mut() {
                            sender.send(delivery.clone()).await.unwrap();
                        }
                    }
                }
            }
        });
    }

    pub fn attach(&mut self, machine: &mut Machine) {
        let (machine_sender, network_receiver) = mpsc::channel(16);
        let (network_sender, machine_receiver) = mpsc::channel(16);
        match self.senders.entry(machine.id()) {
            Entry::Occupied(_) => panic!("Attaching the same machine to the network twice"),
            Entry::Vacant(entry) => {
                entry.insert(network_sender);
            }
        }
        self.receivers.push(network_receiver);
        let info = NetworkInfo {
            mtu: self.mtu,
            sender: machine_sender,
            receiver: machine_receiver,
        };
        machine.attach(info, self.id.into());
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

#[derive(Debug, Clone)]
pub struct Postmarked {
    pub message: Message,
    pub address: PhysicalAddress,
}

#[derive(Debug, Clone)]
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
