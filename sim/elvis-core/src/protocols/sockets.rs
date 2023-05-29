use super::{
    ipv4::ipv4_parsing::Ipv4Header,
    tcp,
    udp::{self, UdpHeader},
    utility::Endpoint,
};
use crate::{
    machine::ProtocolMap,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Udp},
    Control, FxDashMap, Message, Protocol, Session, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
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
    next_fd: RwLock<u64>,
    sockets: Arc<FxDashMap<u64, Arc<Socket>>>,
    socket_sessions: FxDashMap<SocketId, Arc<SocketSession>>,
    listen_bindings: FxDashMap<SocketAddress, u64>,
    notify_init: Notify,
    shutdown: RwLock<Option<Shutdown>>,
}

impl Sockets {
    /// Creates a new instance of the protocol
    pub fn new(local_ipv4_address: Option<Ipv4Address>) -> Self {
        Self {
            local_ipv4_address,
            local_ports: RwLock::new(49152),
            next_fd: RwLock::new(0),
            sockets: Default::default(),
            socket_sessions: Default::default(),
            listen_bindings: Default::default(),
            notify_init: Notify::new(),
            shutdown: Default::default(),
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
        let fd = {
            let mut lock = self.next_fd.write().unwrap();
            let fd = *lock;
            *lock += 1;
            fd
        };
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

    /// Called from Socket::connect() and Socket::accept()
    /// Creates a new socket_session based on IP address and port and returns it
    pub fn open_with_fd(
        &self,
        fd: u64,
        socket_id: SocketId,
        protocols: ProtocolMap,
    ) -> Result<Arc<dyn Session>, OpenError> {
        match self.socket_sessions.entry(socket_id) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing(socket_id))?
            }
            Entry::Vacant(entry) => {
                let downstream = protocols.protocol::<Udp>().unwrap().open_and_listen(
                    TypeId::of::<Self>(),
                    // TODO(hardint): Fix when IPv6 is supported
                    socket_id.try_into().unwrap(),
                    protocols,
                )?;
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(Some(match self.sockets.entry(fd) {
                        Entry::Occupied(sock) => sock.get().clone(),
                        Entry::Vacant(_) => return Err(OpenError::NoSocketForFd(fd)),
                    })),
                    downstream,
                    stored_msg: RwLock::new(None),
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen_with_fd(
        &self,
        fd: u64,
        address: SocketAddress,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        self.listen_bindings.insert(address, fd);
        Ok(protocols
            .protocol::<Udp>()
            .expect("No such protocol")
            // TODO(hardint): Fix when IPv6 is supported
            .listen(TypeId::of::<Self>(), address.try_into().unwrap(), protocols)?)
    }
}

impl Protocol for Sockets {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
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

    /// When the Sockets API receives a message from a Udp or Tcp session, it is
    /// demux'd to the correct socket_session based on IP address and port, the
    /// socket_session will then pass it on to its respective socket
    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let ipv4_header = control.get::<Ipv4Header>().unwrap();
        let udp_header = control.get::<UdpHeader>().unwrap();
        let identifier = SocketId::new_from_addresses(
            SocketAddress::new_v4(ipv4_header.destination, udp_header.destination),
            SocketAddress::new_v4(ipv4_header.source, udp_header.source),
        );
        let any_identifier =
            SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, udp_header.destination);
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("There is already a session for {0:?}")]
    Existing(SocketId),
    #[error("{0}")]
    Udp(#[from] udp::OpenAndListenError),
    #[error("{0}")]
    Tcp(#[from] tcp::OpenError),
    #[error("There was no socket for the file descriptor {0}")]
    NoSocketForFd(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListenError {
    #[error("There is already a session for {0:?}")]
    Existing(Endpoint),
    #[error("{0}")]
    Udp(#[from] udp::ListenError),
    #[error("{0}")]
    Tcp(#[from] tcp::ListenError),
}
