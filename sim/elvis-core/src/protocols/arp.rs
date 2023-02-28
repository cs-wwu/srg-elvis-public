//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//! In ELVIS, the Ipv4Sessions connect with ARP.
//! Arp will fetch MAC addresses when query'd.

pub mod arp_parsing;
pub mod arp_session;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    control::{Control, Key, Primitive},
    machine::PciSlot,
    network::Mac,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::{arp::arp_parsing::ArpPacket, Pci},
    session::SharedSession,
    Id, Message, Network, Protocol, ProtocolMap,
};

use self::arp_session::ArpSession;

use super::{ipv4::Ipv4Address, Ipv4};

use dashmap::{mapref::entry::Entry, DashMap, DashSet};
use tokio::sync::{mpsc::Sender, watch, Barrier};

pub struct Arp {
    /// The ARP table, or cache. Maps Ipv4 addresses to MAC addresses.
    pub arp_table: DashMap<Ipv4Address, Mac>,
    /// Maps PciSlots to sessions. (Just like the PCI below, Arp has one session for each tap slot.)
    sessions: DashMap<PciSlot, Arc<ArpSession>>,
    /// A map of IP addresses to senders. When a MAC is found for one of these IP addresses, the channel will be sent a () signal.
    ip_to_senders: Mutex<DashMap<Ipv4Address, watch::Sender<()>>>,
    /// A set of all this machine's local IPs. Filled by open and listen.
    local_ips: DashSet<Ipv4Address>,
}

impl Arp {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(0x0806);

    /// The time to wait after sending an ARP request before sending another
    pub const RESEND_DELAY: Duration = Duration::from_millis(200);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            arp_table: Default::default(),
            sessions: Default::default(),
            ip_to_senders: Default::default(),
            local_ips: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Returns the MAC address associated with the context's source and destination IP.
    async fn get_mac(self: Arc<Self>, context: &Context) -> Mac {
        // if the mac is in the context, just return that
        let mac_result = Network::get_destination(&context.control);
        if let Ok(mac) = mac_result {
            return mac;
        }

        // if the mac is in the ARP table, just get the mac
        let remote_ip = Ipv4::get_remote_address(&context.control).expect("no IP in context");
        if let Some(mac) = self.arp_table.get(&remote_ip) {
            return *mac;
        }

        // otherwise, get a reciever
        // this reciever will be sent to when the MAC is added to the table
        let mut should_send_requests = false;
        let mut receiver = {
            let ip_to_senders = self.ip_to_senders.lock().expect("could not lock map");
            let entry = ip_to_senders.entry(remote_ip);
            match entry {
                Entry::Occupied(entry) => entry.get().subscribe(),
                Entry::Vacant(entry) => {
                    should_send_requests = true;
                    let (send, _) = watch::channel(());
                    let receiver = send.subscribe();
                    entry.insert(send);
                    receiver
                }
            }
        };

        // wait for the receiver to receive a response
        if should_send_requests {
            // Send requests if it is decided so
            let session = self
                .clone()
                .open_arp(Ipv4::ID, context.control.clone(), context.protocols.clone())
                .expect("Couldn't open session")
                .clone();

            // repeatedly send requests until a response is recieved
            loop {
                session
                    .send_arp_request(context.clone())
                    .expect("unable to send ARP request");
                let timeout = tokio::time::timeout(Self::RESEND_DELAY, receiver.changed());
                let result = timeout.await;
                // If we got a response before the timeout, break
                if let Ok(result) = result {
                    result.expect("got recv error");
                    break;
                }
            }
        } else {
            receiver.changed().await.expect("got recv error");
        }

        // after receiving a response, it is finally time to get the MAC address from the arp table
        *self
            .arp_table
            .get(&remote_ip)
            .expect("There's supposed to be a value in the arp table")
    }

    /// Functions identically to [`Arp::open`], but it returns an Arc<ArpSession> instead of a SharedSession.
    pub fn open_arp(
        self: Arc<Self>,
        _upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<Arc<ArpSession>, OpenError> {
        let pci_slot = Pci::get_pci_slot(&participants).map_err(|_| {
            tracing::error!("Missing PCI slot on context");
            OpenError::MissingContext
        })?;

        // add IP to set of local IPs
        let local_ip = Ipv4::get_local_address(&participants)
            .expect("Missing local IP address in participants");
        self.local_ips.insert(local_ip);

        let result = match self.sessions.entry(pci_slot) {
            Entry::Occupied(entry) => entry.get().clone(),

            Entry::Vacant(entry) => {
                // if there is no session for this tap slot, make a new session
                let downstream = protocols
                    .protocol(Pci::ID)
                    .expect("no such protocol")
                    .open(Arp::ID, participants, protocols)?;

                let result = Arc::new(ArpSession::new(self.clone(), downstream));
                entry.insert(result.clone());
                result
            }
        };

        Ok(result)
    }
}

impl Protocol for Arp {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn open(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        // for some reason, rust wouldn't just let me return the result of open_arp
        let arp_arc = self.open_arp(upstream, participants, protocols)?;
        Ok(arp_arc)
    }

    fn listen(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        assert_eq!(upstream, Ipv4::ID);
        let local = Ipv4::get_local_address(&participants).map_err(|_| {
            tracing::error!("Missing local address on context");
            ListenError::MissingContext
        })?;
        self.local_ips.insert(local);

        // Essentially a no-op but good for completeness and as an example
        protocols
            .protocol(Pci::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)
    }

    /// In general, this will be called by the Pci layer when an ARP packet is recieved
    fn demux(
        self: Arc<Self>,
        message: Message,
        _caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        assert_eq!(Network::get_protocol(&context.control), Ok(Arp::ID));
        let result = ArpPacket::from_bytes(message.iter());
        let packet = result.or(Err(DemuxError::Header))?;

        // If we are not the target for this ARP packet, ignore it. Return early.
        if !self.local_ips.contains(&packet.target_ip) {
            return Ok(());
        }

        // put entry in ARP table and send a message saying we did
        self.arp_table.insert(packet.sender_ip, packet.sender_mac);
        {
            let map_ref = self.ip_to_senders.lock().expect("could not lock map");
            let entry = map_ref.get(&packet.sender_ip);
            if let Some(sender) = entry {
                sender.send(()).expect("failed to send message");
            }
        }

        // If the ARP packet is a request, send a reply
        if packet.is_request {
            let session = self
                .open_arp(Ipv4::ID, context.control, context.protocols.clone())
                .expect("could not open session");
            let mut context = Context::new(context.protocols);
            Ipv4::set_local_address(packet.target_ip, &mut context.control); // we are the target IP
            Ipv4::set_remote_address(packet.sender_ip, &mut context.control);
            Network::set_destination(packet.sender_mac, &mut context.control);
            session
                .send_arp_reply(context)
                .expect("failed to send ARP reply");
        }

        Ok(())
    }

    /// Arp cannot be queried or it will panic.
    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}

impl Default for Arp {
    fn default() -> Self {
        Self::new()
    }
}
