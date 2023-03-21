//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
//! In ELVIS, the Ipv4Sessions connect with ARP.
//! Arp will fetch MAC addresses when query'd.

pub mod arp_parsing;
pub mod arp_session;

use std::{sync::Arc, time::Duration};

use crate::{
    control::{Control, Key, Primitive},
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::Pci,
    session::SharedSession,
    Id, Message, Network, Protocol, ProtocolMap,
};

use self::{arp_parsing::ArpPacket, arp_session::ArpSession, arp_session::MacStatus};

use super::{ipv4::Ipv4Address, Ipv4};

use dashmap::{mapref::entry::Entry, DashMap, DashSet};
use tokio::sync::{mpsc::Sender, Barrier};

pub struct Arp {
    /// Maps destination IP addresses to sessions.
    /// MAC addresses are stored in each session. In a sense, this is the ARP cache.
    sessions: DashMap<Ipv4Address, Arc<ArpSession>>,
    /// A set of all this machine's local IPs. Filled by open and listen.
    local_ips: DashSet<Ipv4Address>,
}

impl Arp {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(0x0806);

    /// The time to wait after sending an ARP request before sending another.
    pub const RESEND_DELAY: Duration = Duration::from_millis(200);

    /// The number of times we should send ARP requests before giving up.
    pub const RESEND_TRIES: i32 = 10;

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            sessions: Default::default(),
            local_ips: Default::default(),
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
        _upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let local_ip = Ipv4::get_local_address(&participants).map_err(|_| {
            tracing::error!("Missing local IP address on context");
            OpenError::MissingContext
        })?;

        let remote_ip = Ipv4::get_remote_address(&participants).map_err(|_| {
            tracing::error!("Missing remote IP address on context");
            OpenError::MissingContext
        })?;

        self.local_ips.insert(local_ip);

        let result = match self.sessions.entry(remote_ip) {
            Entry::Occupied(entry) => entry.get().clone(),

            Entry::Vacant(entry) => {
                // if there is no session for this IP address, make a new session
                let downstream = protocols
                    .protocol(Pci::ID)
                    .expect("no such protocol")
                    .open(Arp::ID, participants.clone(), protocols.clone())?;

                let remote_mac = Network::get_destination(&participants).ok();

                let session = Arc::new(ArpSession::new(remote_ip, remote_mac, downstream));

                // IMPORTANT: we gotta send ARP requests so the MAC address of the session can get set
                tokio::spawn(
                    session
                        .clone()
                        .send_arp_requests(local_ip, protocols.clone()),
                );
                entry.insert(session.clone());

                session
            }
        };

        Ok(result)
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
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
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        let result = ArpPacket::from_bytes(message.iter());
        let packet = result.or(Err(DemuxError::Header))?;

        // If we are not the target for this ARP packet, ignore it. Return early.
        if !self.local_ips.contains(&packet.target_ip) {
            return Ok(());
        }

        // put entry in ARP table and send a message saying we did
        match self.sessions.entry(packet.sender_ip) {
            // If we already have an entry for this session, set its status
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                let packet = packet;
                // spawn a thread to set this session's status
                tokio::spawn(session.set_status(MacStatus::Set(packet.sender_mac)));
            }
            // If we don't have an entry for this session, make one
            Entry::Vacant(entry) => {
                let dest_ip = packet.sender_ip;
                let dest_mac = packet.sender_mac;
                let session = Arc::new(ArpSession::new(dest_ip, Some(dest_mac), caller));
                entry.insert(session);
            }
        }

        // If the packet was an arp request, send an arp reply
        if packet.is_request {
            let session = { self.sessions.get(&packet.sender_ip).unwrap().clone() };
            // Notice that we've said the local_ip is packet.target_ip. This is because we were the target of the packet.
            let _ = session.send_arp_reply(packet.target_ip, packet.sender_mac, context.protocols);
        }

        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}

impl Default for Arp {
    fn default() -> Self {
        Self::new()
    }
}
