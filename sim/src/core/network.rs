use futures::stream::{FuturesUnordered, StreamExt};
use std::mem;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::protocols::tap::NetworkInfo;

use super::{message::Message, Machine, MachineId};
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

/// A maximum transmission unit
pub type Mtu = u32;

pub type NetworkId = u32;

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

    pub fn start(&mut self, _shutdown: Sender<()>) {
        let mut receivers = mem::take(&mut self.receivers);
        let id = self.id;
        let mut senders = mem::take(&mut self.senders);
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
                        for sender in senders
                            .iter_mut()
                            .filter_map(|(&id, sender)| (id != next.sender).then_some(sender))
                        {
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
        machine.attach(info, self.id);
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
    pub sender: MachineId,
}

#[derive(Debug, Clone)]
pub struct Delivery {
    pub message: Message,
    pub network: NetworkId,
}
