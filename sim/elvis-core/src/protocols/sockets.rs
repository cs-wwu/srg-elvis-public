use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::{ipv4::Ipv4Address, Ipv4, Udp},
    session::SharedSession,
    Control, FxDashMap, Id, Message, Protocol, ProtocolMap, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

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
    shutdown: RwLock<Option<Shutdown>>,
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
            shutdown: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Creates a new socket and adds it to its listing of sockets
    pub fn new_socket(
        self: Arc<Self>,
        domain: ProtocolFamily,
        sock_type: SocketType,
        protocols: ProtocolMap,
    ) -> Result<Arc<Socket>, SocketError> {
        let fd = Id::new(*self.fds.read().unwrap());
        let socket = Arc::new(Socket::new(
            domain,
            sock_type,
            fd,
            protocols,
            self.clone(),
            self.shutdown.read().unwrap().as_ref().unwrap().clone(),
        ));
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

    fn get_local_ipv4(self: Arc<Self>) -> Result<IpAddress, SocketError> {
        match self.local_ipv4_address {
            Some(v) => Ok(IpAddress::IPv4(v)),
            None => Err(SocketError::Other),
        }
    }

    fn get_ephemeral_port(self: Arc<Self>) -> Result<u16, SocketError> {
        let port = *self.local_ports.read().unwrap();
        *self.local_ports.write().unwrap() += 1;
        Ok(port)
    }

    fn get_ephemeral_endpoint(self: Arc<Self>) -> Result<SocketAddress, SocketError> {
        Ok(SocketAddress {
            address: self.clone().get_local_ipv4()?,
            port: self.get_ephemeral_port()?,
        })
    }

    fn get_socket_session(
        self: Arc<Self>,
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
                    upstream: RwLock::new(Some(upstream)),
                    downstream,
                    sockets: self.sockets.clone(),
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        self: Arc<Self>,
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
        self: Arc<Self>,
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
                    sockets: self.sockets.clone(),
                });
                socket.add_listen_address(identifier.remote_address);
                entry.insert(session.clone());
                session
            }
        };
        session.receive(message)?;
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
