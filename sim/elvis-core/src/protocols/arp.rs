//! Address resolution protocol (ARP) is used by computers to associate IP
//! addresses with MAC addresses.
pub mod arp_parsing;
pub mod arp_session;

use std::{sync::Arc, time::Duration};

use crate::{
    control::{Control, Key, Primitive},
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::Pci,
    session::SharedSession,
    Id, Message, Network, Protocol, ProtocolMap, Shutdown,
};

use self::{arp_parsing::ArpPacket, arp_session::ArpSession, arp_session::MacStatus};

use super::{ipv4::Ipv4Address, Ipv4};

use dashmap::{mapref::entry::Entry, DashMap, DashSet};
use tokio::sync::Barrier;

/// Arp stands for Address Resolution Protocol. Its job is to figure out another (Ipv4-using) machine's MAC
/// address, and send messages to that MAC, instead of broadcasting them to the whole network.
///
/// In ELVIS, Arp sits (optionally) between the Ipv4 and Pci protocols.
/// Using Arp is rather simple. Just add it to your machine.
///
/// ```compile_fail
/// Machine::new([
///     Udp::new().shared() as SharedProtocol,
///     Ipv4::new(std::iter::empty().collect()).shared(),
///     Arp::new().shared(),
///     Pci::new([]).shared(),
/// ])
/// ```
///
/// ARP will attach destination MAC addresses to a message's Context, if it is not set.
///
/// The machine you are sending messages to MUST also have an Arp protocol, and a local IP address
/// (set by [`Ipv4::open`] or [`Ipv4::listen`]).
#[derive(Default)]
pub struct Arp {
    /// Maps destination IP addresses to sessions.
    /// destination MAC addresses are stored in each session. In a sense, this is the ARP cache.
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
    pub const RESEND_TRIES: u32 = 10;

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Protocol for Arp {
    fn id(&self) -> Id {
        Self::ID
    }

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    /// Opens an ARP session.
    /// The participants set must contain:
    /// - A local IP address ([`Ipv4::set_local_address`])
    /// - A remote IP address ([`Ipv4::set_remote_address`])
    /// - A pci slot ([`Pci::set_pci_slot`])
    ///
    /// If a destination MAC address is specified ([`Network::set_destination`]),
    /// then ARP requests will not be sent out (because the MAC is already resolved).
    ///
    /// When an ARP session is opened, it will attempt to associate a MAC address with the remote IP address,
    /// by sending ARP requests.
    ///
    /// The other machine MUST also have Arp, and a local IP address set by [`Arp::open`] or [`Arp::listen`].
    ///
    /// # MAC address resolution (complicated technical details)
    ///
    /// The ARP requests will be repeatedly sent until an ARP reply is received,
    /// or until [`RESEND_TRIES`] is reached. There is a delay of [`RESEND_DELAY`]
    /// between each time a packet is sent.
    fn open(
        &self,
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

    /// If the context contains a local IP address (set by [`Ipv4::set_local_address`]),
    /// then it will be added to the list of this machine's local IP addresses.
    ///
    /// (This is important, because other machines with Arp can't resolve this machine's MAC address
    /// unless it has an IP address.)
    fn listen(
        &self,
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

    /// In general, this will be called by the Pci layer when an ARP packet is recieved.
    ///
    /// The demux method follows these steps when it receives a message:
    ///
    /// 1. Attempt to parse the ArpPacket. Return [`DemuxError::Header`] if the ArpPacket could not be parsed.
    /// 2. Check if we are the target for this ARP packet. If not, return Ok(()).
    /// 3. Add an IP address to MAC address mapping, based on the sender IP and sender MAC.
    /// 4. If the message was an ARP request, send back an ARP reply to the other machine.
    fn demux(
        &self,
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
                // set this session's status
                session
                    .dest_mac
                    .send_replace(MacStatus::Set(packet.sender_mac));
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

        tracing::info!(
            "ARP: Resolved IP {} -> {} for machine with IP {}",
            packet.sender_ip,
            packet.sender_mac,
            packet.target_ip
        );

        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
