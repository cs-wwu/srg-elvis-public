//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//! In ELVIS, the Ipv4Sessions connect with ARP.
//! Arp will fetch MAC addresses when query'd.

use std::{sync::Arc, collections::HashMap};

use crate::{ProtocolMap, Message};
use crate::session::SharedSession;
use crate::{network::Mac, Id, Protocol};

use crate::protocol::{QueryError, Context, ListenError, StartError, OpenError, DemuxError};

use crate::control::{Primitive, Control, Key};

use super::ipv4::Ipv4Address;

use tokio::sync::mpsc::Sender;
use tokio::sync::{Barrier};

pub struct Arp {
    /// The ARP table, or cache. Maps Ipv4 addresses to MAC addresses.
    pub arp_table: HashMap<Ipv4Address, Mac>,
}

impl Arp {
    /// A unique identifier for the protocol. (0x0806 is the EtherType value of ARP)
    pub const ID: Id = Id::new(0x0806);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            arp_table: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Protocol for Arp {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn start(
        self: Arc<Self>,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: crate::ProtocolMap,
    ) -> Result<(), StartError> {
        todo!()
    }

    fn open(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        unimplemented!("Cannot open on an Arp");
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        Ok(())
    }

    /// In general, this will be called by the Pci layer when an ARP packet is recieved
    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        todo!()
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
    /// Returns `Ok(Primitive::U64(result_mac))`
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        // TODO: Perhaps the ARP table could be changed to map from u32 to Mac?
        // It would be faster but less readable
        let ip_addr: Ipv4Address = Ipv4Address::from(key.1 as u32);
        let result_mac = match self.arp_table.get(&ip_addr) {
            Some(result_mac) => *result_mac,
            None => {
                // if the MAC is not in the table, send out an ARP request
                // and wait for response
                todo!()
            },
        };
        Ok(Primitive::U64(result_mac))
    }
}