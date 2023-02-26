use std::sync::Arc;

use dashmap::mapref::entry::Entry;

use crate::{
    control::{Key, Primitive},
    machine::PciSlot,
    protocols::{Pci, ipv4::Ipv4Address},
    session::{QueryError, SharedSession},
    Session,
};

use super::{Arp, arp_parsing::ArpPacket};

pub struct ArpSession {
    /// The PCI slot to send ARP requests through.
    slot: PciSlot,
    /// The local IP address of this machine.
    local_ip: Ipv4Address,
    /// The IP address to request a MAC for.
    remote_ip: Ipv4Address,
    /// The ARP protocol object that created this session.
    parent: Arc<Arp>,
    /// The Pci session to send messages through
    downstream: SharedSession,
}

impl ArpSession {
    pub fn new(
        slot: PciSlot,
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        parent: Arc<Arp>,
        downstream: SharedSession,
    ) -> Self {
        ArpSession {
            slot,
            local_ip,
            remote_ip,
            parent,
            downstream,
        }
    }
}

impl Session for ArpSession {
    fn send(
        self: Arc<Self>,
        _message: crate::Message,
        _context: crate::protocol::Context,
    ) -> Result<(), crate::session::SendError> {
        unimplemented!("Cannot send on ArpSession");
    }

    /// Returns the MAC address associated with the given Ipv4 address.
    /// Sends out an ARP request to get the Ipv4 address, if necessary.
    ///
    /// # Arguments
    ///
    /// * `key` - a Key of the form (_, Ipv4Address)
    ///
    /// # Returns
    ///
    /// `Ok(Primitive::U64(result_mac))`
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        let ip_address = Ipv4Address::from(key.1 as u32);
        let arp_entry = self.parent.arp_table.entry(ip_address);

        let mac = match arp_entry {
            Entry::Occupied(mac_entry) => *mac_entry.get(),
            Entry::Vacant(mac_entry) => {
                // send out ARP request
                todo!();
            }
        };
        Ok(Primitive::U64(mac))
    }
}
