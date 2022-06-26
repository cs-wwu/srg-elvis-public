use std::collections::{hash_map::Entry, HashMap};

use super::Message;

pub type Mtu = u32;
pub type Mac = usize;

type Pending = HashMap<usize, Vec<Message>>;

#[derive(Debug, Clone)]
pub struct Network {
    mtu: Mtu,
    connected: Vec<Mac>,
    pending: Pending,
}

impl Network {
    pub fn new(connected: Vec<Mac>, mtu: Mtu) -> Self {
        Self {
            connected,
            pending: Default::default(),
            mtu,
        }
    }

    pub fn mtu(&self) -> Mtu {
        self.mtu
    }

    pub fn connected_machines(&self) -> &[Mac] {
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

    // Todo: Allow only taking individual messages as a speed control mechanism
    pub fn take_queue(&mut self, address: Mac) -> Vec<Message> {
        match self.pending.entry(address) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => vec![],
        }
    }
}

fn send_to_mac(mac: Mac, pending: &mut Pending, message: Message) {
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
    Mac(Mac),
    Broadcast,
}
