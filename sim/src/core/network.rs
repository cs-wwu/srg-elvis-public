use super::{message::Message, MachineId};
use std::collections::{hash_map::Entry, HashMap};

/// A maximum transmission unit
pub type Mtu = u32;

type Pending = HashMap<usize, Vec<Message>>;

// Todo: Explore having Network hold Machine instances

/// A link-level connection between [`Machine`](super::Machine)s.
///
/// A network facilitates connecting multiple machines together and allowing
/// them to exchange [`Message`]s. Roughly, it models an simplified Ethernet
/// network with broadcast and MAC-based message delivery.
#[derive(Debug, Clone)]
pub struct Network {
    mtu: Mtu,
    connected: Vec<MachineId>,
    pending: Pending,
}

impl Network {
    /// Create a new network with the given `mtu` and list of networked
    /// [`Machine`](super::Machine)s.
    pub fn new(connected: Vec<MachineId>, mtu: Mtu) -> Self {
        Self {
            connected,
            pending: Default::default(),
            mtu,
        }
    }

    /// The network's maximum transmission unit.
    pub fn mtu(&self) -> Mtu {
        self.mtu
    }

    /// The list of connected machines.
    pub fn connected_machines(&self) -> &[MachineId] {
        &self.connected
    }

    /// Send a `message` to the machine or machines identified by `address`.
    pub fn send(&mut self, address: PhysicalAddress, message: Message) {
        // Todo: Check that the message is shorter than MTU
        match address {
            PhysicalAddress::Recipient(mac) => send_to_mac(mac, &mut self.pending, message),
            PhysicalAddress::Broadcast => {
                for &mac in self.connected.iter() {
                    send_to_mac(mac, &mut self.pending, message.clone())
                }
            }
        }
    }

    /// Remove and return the list messages not yet processed that are destined
    /// for delivery to `address`.
    pub fn take_queue(&mut self, address: MachineId) -> Vec<Message> {
        // Todo: Allow only taking individual messages as a speed control
        // mechanism
        match self.pending.entry(address) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => vec![],
        }
    }
}

fn send_to_mac(mac: MachineId, pending: &mut Pending, message: Message) {
    match pending.entry(mac) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().push(message);
        }
        Entry::Vacant(entry) => {
            entry.insert(vec![message]);
        }
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
