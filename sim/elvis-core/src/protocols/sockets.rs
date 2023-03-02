use dashmap::{mapref::entry::Entry, DashMap};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc::Sender, Barrier};

use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::{ipv4::Ipv4Address, Ipv4, Udp},
    session::SharedSession,
    Control, Id, Message, Protocol, ProtocolMap,
};

pub mod socket;
use socket::{IpAddress, ProtocolFamily, Socket, SocketError, SocketId, SocketType};
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
    _local_ipv4_address: Option<Ipv4Address>,
    // TODO(giddinl2): This will be added once IPv6 is implemented
    // local_ipv6_address: Option<Ipv6Address>,
    fds: RwLock<u64>,
    sockets: DashMap<Id, Arc<Socket>>,
    socket_sessions: DashMap<SocketId, Arc<SocketSession>>,
}

impl Sockets {
    /// A unique identifier for the protocol
    pub const ID: Id = Id::from_string("Sockets");

    /// Creates a new instance of the protocol
    pub fn new(_local_ipv4_address: Option<Ipv4Address>) -> Self {
        Self {
            _local_ipv4_address,
            fds: RwLock::new(0),
            sockets: Default::default(),
            socket_sessions: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared(ipv4_address: Option<Ipv4Address>) -> Arc<Self> {
        Arc::new(Self::new(ipv4_address))
    }

    /// Creates a new socket and adds it to its listing of sockets
    pub fn new_socket(
        self: Arc<Self>,
        domain: ProtocolFamily,
        sock_type: SocketType,
        protocols: ProtocolMap,
    ) -> Result<Arc<Socket>, SocketError> {
        let fd = Id::new(*self.fds.read().unwrap());
        let socket = Arc::new(Socket::new(domain, sock_type, fd, protocols));
        match self.sockets.entry(fd) {
            Entry::Occupied(_) => {
                return Err(SocketError::Other(String::from(
                    "Failed to create new Socket",
                )))
            }
            Entry::Vacant(entry) => entry.insert(socket.clone()),
        };
        // Currently, mock "file descriptors" are distrubuted on an incremental
        // basis and not reused
        *self.fds.write().unwrap() += 1;
        Ok(socket)
    }
}

impl Protocol for Sockets {
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

    /// Called from Socket::connect() and Socket::accept()
    /// Creates a new socket_session based on IP address and port and returns it
    fn open(
        self: Arc<Self>,
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
                    upstream: match self.sockets.entry(upstream) {
                        Entry::Occupied(entry) => entry.get().clone(),
                        Entry::Vacant(_) => return Err(OpenError::MissingContext),
                    },
                    downstream,
                    //id: identifier
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        todo!()
    }

    /// When the Sockets API receives a message from a Udp or Tcp session, it is
    /// demux'd to the correct socket_session based on IP address and port, the
    /// socket_session will then pass it on to its respective socket
    fn demux(
        self: Arc<Self>,
        message: Message,
        _caller: SharedSession,
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
        match self.socket_sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone().receive(message, context),
            Entry::Vacant(_) => {
                tracing::error!("Tried to demux with a missing session and no listen bindings");
                Err(DemuxError::MissingSession)?
            }
        }
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
