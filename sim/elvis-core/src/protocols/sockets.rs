use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError, SharedProtocol},
    protocols::{ipv4::Ipv4Address, Ipv4, Udp, Dns},
    session::SharedSession,
    Control, FxDashMap, Id, Message, Protocol, ProtocolMap, Shutdown, Machine
};
use dashmap::mapref::entry::Entry;
use std::sync::{Arc, RwLock};
use tokio::sync::{Barrier, Notify};

pub mod socket;
use socket::{IpAddress, ProtocolFamily, Socket, SocketAddress, SocketError, SocketId, SocketType};

mod socket_session;
use socket_session::SocketSession;

/// An implementation of the Sockets API
///
/// Creates, distributes, and tracks [`Socket`]s on a given [`Machine`]
///
/// Purpose:
/// - To serve as an interface between the x-kernal-style protocol stack and a
/// unix-style application
/// - To simplify the process of making connections via the protocol stack to
/// make applications easier to write
#[derive(Default)]
pub struct Sockets {
    // TODO(giddinl2): This will be used once I figure out how to dynamically hand out unused ports
    local_ipv4_address: Option<Ipv4Address>,
    local_ports: RwLock<u16>,
    // TODO(giddinl2): This will be added once IPv6 is implemented
    // local_ipv6_address: Option<Ipv6Address>,
    fds: RwLock<u64>,
    sockets: Arc<FxDashMap<Id, Arc<Socket>>>,
    socket_sessions: FxDashMap<SocketId, Arc<SocketSession>>,
    listen_bindings: FxDashMap<SocketAddress, Id>,
    notify_init: Notify,
    shutdown: RwLock<Option<Shutdown>>,
    // Sockets use Dns as a tool for looking up IPs and need direct access
    // to the Dns cache.
    dns: Dns,
}

impl Sockets {
    /// A unique identifier for the protocol
    pub const ID: Id = Id::from_string("Sockets");

    /// Creates a new instance of the protocol
    pub fn new(local_ipv4_address: Option<Ipv4Address>) -> Self {
        Self {
            local_ipv4_address,
            local_ports: RwLock::new(49152),
            fds: RwLock::new(0),
            sockets: Default::default(),
            socket_sessions: Default::default(),
            listen_bindings: Default::default(),
            notify_init: Notify::new(),
            shutdown: Default::default(),
            dns: Dns::new(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Creates a new socket and adds it to its listing of sockets
    pub async fn new_socket(
        self: &Arc<Self>,
        domain: ProtocolFamily,
        sock_type: SocketType,
        protocols: ProtocolMap,
    ) -> Result<Arc<Socket>, SocketError> {
        let fd = Id::new(*self.fds.read().unwrap());
        self.notify_init.notified().await;
        let socket = Arc::new(Socket::new(
            domain,
            sock_type,
            fd,
            protocols,
            self.clone(),
            self.shutdown.read().unwrap().as_ref().unwrap().clone(),
        ));
        self.notify_init.notify_one();
        match self.sockets.entry(fd) {
            Entry::Occupied(_) => {
                return Err(SocketError::Other);
            }
            Entry::Vacant(entry) => entry.insert(socket.clone()),
        };
        // Currently, mock "file descriptors" are distrubuted on an incremental
        // basis and not reused
        *self.fds.write().unwrap() += 1;
        Ok(socket)
    }

    fn get_local_ipv4(&self) -> Result<IpAddress, SocketError> {
        match self.local_ipv4_address {
            Some(v) => Ok(IpAddress::IPv4(v)),
            None => Err(SocketError::Other),
        }
    }

    fn get_ephemeral_port(&self) -> Result<u16, SocketError> {
        let port = *self.local_ports.read().unwrap();
        *self.local_ports.write().unwrap() += 1;
        Ok(port)
    }

    fn get_ephemeral_endpoint(&self) -> Result<SocketAddress, SocketError> {
        Ok(SocketAddress {
            address: self.get_local_ipv4()?,
            port: self.get_ephemeral_port()?,
        })
    }

    fn get_socket_session(
        &self,
        local: SocketAddress,
        remote: SocketAddress,
    ) -> Result<Arc<SocketSession>, SocketError> {
        let listen_local = SocketAddress::new_v4(self.local_ipv4_address.unwrap(), local.port);
        let listen_identifier = SocketId::new_from_addresses(listen_local, remote);
        let session = match self.socket_sessions.entry(listen_identifier) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => {
                return Err(SocketError::AcceptError);
            }
        };
        let identifier = SocketId::new(local.address, local.port, remote.address, remote.port);
        self.socket_sessions.insert(identifier, session.clone());
        Ok(session)
    }

    pub(crate) fn forward_to_socket(
        self: Arc<Self>,
        fd: Id,
        message: Message,
        _context: Context,
    ) -> Result<(), DemuxError> {
        match self.sockets.entry(fd) {
            Entry::Occupied(entry) => entry.get().receive(message),
            Entry::Vacant(_) => Err(DemuxError::MissingSession),
        }
    }
    
    /// Finds the IP associated with the given domain name.
    fn get_host_by_name(
        &self,
        name: String,
        protocols: ProtocolMap,
    ) -> Result<Ipv4Address, SocketError> {
        // Get DNS protocol from this socket protocol's machine
        // let dns: Dns =  match protocols.protocol(Dns::ID) {
        //     Some(p) => p,
        //     None => {
        //         return Err(SocketError::Other);
        //     }
        // };

        match self.dns.get_mapping(name) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(DnsError) => {
                // TODO(zachd9757): Check authoritative server
                Err(SocketError::Other)
            },
        }
    }
}

impl Protocol for Sockets {
    fn id(&self) -> Id {
        Self::ID
    }

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        *self.shutdown.write().unwrap() = Some(shutdown);
        self.notify_init.notify_one();
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    /// Called from Socket::connect() and Socket::accept()
    /// Creates a new socket_session based on IP address and port and returns it
    fn open(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let identifier = SocketId::new(
            IpAddress::IPv4(Ipv4::get_local_address(&participants).map_err(|_| {
                tracing::error!("Missing local address on context");
                OpenError::MissingContext
            })?),
            Udp::get_local_port(&participants).map_err(|_| {
                tracing::error!("Missing local port on context");
                OpenError::MissingContext
            })?,
            IpAddress::IPv4(Ipv4::get_remote_address(&participants).map_err(|_| {
                tracing::error!("Missing remote address on context");
                OpenError::MissingContext
            })?),
            Udp::get_remote_port(&participants).map_err(|_| {
                tracing::error!("Missing remote port on context");
                OpenError::MissingContext
            })?,
        );
        match self.socket_sessions.entry(identifier) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing)?
            }
            Entry::Vacant(entry) => {
                let downstream = protocols
                    .protocol(Udp::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols)?;
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(Some(match self.sockets.entry(upstream) {
                        Entry::Occupied(sock) => sock.get().clone(),
                        Entry::Vacant(_) => return Err(OpenError::MissingProtocol(upstream)),
                    })),
                    downstream,
                    stored_msg: RwLock::new(None),
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        let identifier = SocketAddress::new_v4(
            Ipv4::get_local_address(&participants).map_err(|_| {
                tracing::error!("Missing local address on context");
                ListenError::MissingContext
            })?,
            Udp::get_local_port(&participants).map_err(|_| {
                tracing::error!("Missing local port on context");
                ListenError::MissingContext
            })?,
        );
        self.listen_bindings.insert(identifier, upstream);
        protocols
            .protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)
    }

    /// When the Sockets API receives a message from a Udp or Tcp session, it is
    /// demux'd to the correct socket_session based on IP address and port, the
    /// socket_session will then pass it on to its respective socket
    fn demux(
        &self,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        let identifier = SocketId::new(
            IpAddress::IPv4(Ipv4::get_local_address(&context.control).map_err(|_| {
                tracing::error!("Missing local address on context");
                DemuxError::MissingContext
            })?),
            Udp::get_local_port(&context.control).map_err(|_| {
                tracing::error!("Missing local port on context");
                DemuxError::MissingContext
            })?,
            IpAddress::IPv4(Ipv4::get_remote_address(&context.control).map_err(|_| {
                tracing::error!("Missing remote address on context");
                DemuxError::MissingContext
            })?),
            Udp::get_remote_port(&context.control).map_err(|_| {
                tracing::error!("Missing remote port on context");
                DemuxError::MissingContext
            })?,
        );
        let any_identifier =
            SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, identifier.local_address.port);
        let session = match self.socket_sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let binding = match self.listen_bindings.get(&identifier.local_address) {
                    Some(listen_entry) => listen_entry,
                    // If we don't have a normal listen binding, check for
                    // a 0.0.0.0 binding
                    None => match self.listen_bindings.get(&any_identifier) {
                        Some(any_listen_entry) => any_listen_entry,
                        None => {
                            tracing::error!(
                                "Tried to demux with a missing session and no listen bindings"
                            );
                            Err(DemuxError::MissingSession)?
                        }
                    },
                };
                let socket = match self.sockets.entry(*binding) {
                    Entry::Occupied(entry) => entry.get().clone(),
                    Entry::Vacant(_) => return Err(DemuxError::MissingSession),
                };
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(None),
                    downstream: caller,
                    stored_msg: RwLock::new(Some(message.clone())),
                });
                socket.add_listen_address(identifier.remote_address);
                entry.insert(session.clone());
                session
            }
        };
        session.receive(message)?;
        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// Test for Sockets:get_host_by_name() when Dns cache is empty
    async fn ghbn_cache_miss() {
        let sockets = Sockets::new(None).shared();

        let machine: Machine = 
            Machine::new([
                sockets.clone() as SharedProtocol,
            ]);

        let shutdown = Shutdown::new();
        let total_protocols: usize = machine.protocol_count();
        let initialized = Arc::new(Barrier::new(total_protocols));
        let protocols: ProtocolMap = machine.protocols.clone();
        
        machine.start(shutdown.clone(), initialized.clone());
        

        let ip: Result<Ipv4Address, SocketError> =
            sockets.get_host_by_name("DNE".to_string(), protocols);

        assert_eq!(ip, Err(SocketError::Other));
    }


    #[tokio::test]
    /// Test for Sockets:get_host_by_name() when IP is found in Dns cache
    async fn ghbn_cache_hit() {
        let sockets = Sockets::new(None).shared();

        let machine: Machine = 
            Machine::new([
                sockets.clone() as SharedProtocol,
            ]);

        let shutdown = Shutdown::new();
        let total_protocols: usize = machine.protocol_count();
        let initialized = Arc::new(Barrier::new(total_protocols));
        let protocols: ProtocolMap = machine.protocols.clone();
        
        machine.start(shutdown.clone(), initialized.clone());
        

        let ip: Result<Ipv4Address, SocketError> =
            sockets.get_host_by_name("DNE".to_string(), protocols);

        assert_eq!(ip, Err(SocketError::Other));
    }
}
