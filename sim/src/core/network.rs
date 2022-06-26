use std::collections::{hash_map::Entry, HashMap};

use super::Message;

type Pending = HashMap<usize, Vec<Message>>;

#[derive(Debug, Clone)]
pub struct Network {
    connected: Vec<usize>,
    pending: Pending,
}

impl Network {
    pub fn new(connected: Vec<usize>) -> Self {
        Self {
            connected,
            pending: Default::default(),
        }
    }

    pub fn connected_machines(&self) -> &[usize] {
        &self.connected
    }

    pub fn send(&mut self, address: PhysicalAddress, message: Message) {
        match address {
            PhysicalAddress::Mac(mac) => send_to_mac(mac, &mut self.pending, message),
            PhysicalAddress::Broadcast => {
                for &mac in self.connected.iter() {
                    send_to_mac(mac, &mut self.pending, message.clone())
                }
            }
        }
    }

    pub fn take_queue(&mut self, address: usize) -> Vec<Message> {
        match self.pending.entry(address) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => vec![],
        }
    }
}

fn send_to_mac(mac: usize, pending: &mut Pending, message: Message) {
    match pending.entry(mac) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().push(message);
        }
        Entry::Vacant(entry) => {
            entry.insert(vec![message]);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalAddress {
    Mac(usize),
    Broadcast,
}
