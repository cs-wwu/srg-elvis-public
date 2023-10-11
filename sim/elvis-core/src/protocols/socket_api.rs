use super::{
    ipv4::ipv4_parsing::Ipv4Header,
    tcp,
    udp::{self, UdpHeader},
    utility::Endpoint,
    Endpoints, Tcp,
};
use crate::{
    machine::ProtocolMap,
    protocol::{DemuxError, NotifyType, StartError},
    protocols::{ipv4::Ipv4Address, Udp},
    Control, FxDashMap, Message, Protocol, Session, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{
    any::TypeId,
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use tokio::sync::{Barrier, Notify};

pub mod socket;
use socket::{ProtocolFamily, Socket, SocketError, SocketType};

mod socket_session;
use socket_session::SocketSession;

/// An implementation of the Sockets API
///
/// Creates, distributes, and tracks [`Socket`]s on a given [`Machine`](crate::machine::Machine)
///
/// Purpose:
/// - To serve as an interface between the x-kernal-style protocol stack and a
/// unix-style application
/// - To simplify the process of making connections via the protocol stack to
/// make applications easier to write
#[derive(Default)]
pub struct SocketAPI {
    local_address: Option<Ipv4Address>,
    local_ports: RwLock<u16>,
    next_fd: RwLock<u64>,
    sockets: Arc<FxDashMap<u64, Arc<Socket>>>,
    socket_sessions: FxDashMap<Endpoints, Arc<SocketSession>>,
    listen_bindings: FxDashMap<Endpoint, u64>,
    notify_init: Notify,
    shutdown: RwLock<Option<Shutdown>>,
}

impl SocketAPI {
    /// Creates a new instance of the protocol
    pub fn new(local_address: Option<Ipv4Address>) -> Self {
        Self {
            local_address,
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

    fn get_local_ip(&self) -> Result<Ipv4Address, SocketError> {
        match self.local_address {
            Some(v) => Ok(v),
            None => Err(SocketError::Other),
        }
    }

    fn get_ephemeral_port(&self) -> Result<u16, SocketError> {
        let port = *self.local_ports.read().unwrap();
        *self.local_ports.write().unwrap() += 1;
        Ok(port)
    }

    fn get_ephemeral_endpoint(&self) -> Result<Endpoint, SocketError> {
        Ok(Endpoint {
            address: self.get_local_ip()?,
            port: self.get_ephemeral_port()?,
        })
    }

    fn get_socket_session(
        &self,
        local: Endpoint,
        remote: Endpoint,
    ) -> Result<Arc<SocketSession>, SocketError> {
        let listen_local = Endpoint::new(self.local_address.unwrap(), local.port);
        let listen_identifier = Endpoints::new(listen_local, remote);
        let session = match self.socket_sessions.entry(listen_identifier) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => {
                return Err(SocketError::AcceptError);
            }
        };
        let identifier = Endpoints::new(local, remote);
        self.socket_sessions.insert(identifier, session.clone());
        Ok(session)
    }

    /// Called from Socket::connect() and Socket::accept()
    /// Creates a new socket_session based on IP address and port and returns it
    pub async fn open_with_fd(
        &self,
        fd: u64,
        socket_id: Endpoints,
        protocols: ProtocolMap,
    ) -> Result<Arc<dyn Session>, OpenError> {
        match self.socket_sessions.entry(socket_id) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing(socket_id))?
            }
            Entry::Vacant(entry) => {
                let sock = match self.sockets.entry(fd) {
                    Entry::Occupied(sock) => sock.get().clone(),
                    Entry::Vacant(_) => return Err(OpenError::NoSocketForFd(fd)),
                };
                let downstream = match sock.sock_type {
                    SocketType::Datagram => {
                        protocols
                            .protocol::<Udp>()
                            .unwrap()
                            .open_and_listen(TypeId::of::<Self>(), socket_id, protocols)
                            .await?
                    }
                    SocketType::Stream => {
                        protocols
                            .protocol::<Tcp>()
                            .unwrap()
                            .open(TypeId::of::<Self>(), socket_id, protocols)
                            .await?
                    }
                };
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(Some(sock)),
                    downstream,
                    stored_messages: RwLock::new(VecDeque::new()),
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen_with_fd(
        &self,
        fd: u64,
        address: Endpoint,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        let sock = match self.sockets.entry(fd) {
            Entry::Occupied(sock) => sock.get().clone(),
            Entry::Vacant(_) => return Err(ListenError::NoSocketForFd(fd)),
        };
        if self.listen_bindings.get(&address).is_some() {
            return Err(ListenError::Existing(address));
        }
        self.listen_bindings.insert(address, fd);
        match sock.sock_type {
            SocketType::Datagram => Ok(protocols
                .protocol::<Udp>()
                .expect("No such protocol")
                .listen(TypeId::of::<Self>(), address, protocols)?),
            SocketType::Stream => Ok(protocols
                .protocol::<Tcp>()
                .expect("No such protocol")
                .listen(TypeId::of::<Self>(), address, protocols)?),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for SocketAPI {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Listen on the local IP address.
        // This is necessary for ARP purposes.
        // A machine must set its local IP so it can tell other machines about itself.
        // TODO(sudobeans): SocketAPI shouldn't need to know about ARP!
        // Seems like code smell (on ARP's part).
        if let Some(local_address) = self.local_address {
            if let Some(arp) = protocols.protocol::<crate::protocols::Arp>() {
                arp.listen(local_address);
            }
        }

        *self.shutdown.write().unwrap() = Some(shutdown);
        self.notify_init.notify_one();
        initialized.wait().await;
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
        let identifier = match control.get::<UdpHeader>() {
            Some(udp_header) => {
                let ipv4_header = control.get::<Ipv4Header>().unwrap();
                Endpoints::new(
                    Endpoint::new(ipv4_header.destination, udp_header.destination),
                    Endpoint::new(ipv4_header.source, udp_header.source),
                )
            }
            None => match control.get::<Endpoints>() {
                Some(endpoints) => *endpoints,
                None => return Err(DemuxError::Header),
            },
        };
        let any_identifier = Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local.port);
        match self.socket_sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().receive(message)?,
            Entry::Vacant(entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let binding = match self.listen_bindings.get(&identifier.local) {
                    Some(listen_entry) => listen_entry,
                    // If we don't have a normal listen binding, check for
                    // a 0.0.0.0 binding
                    None => match self.listen_bindings.get(&any_identifier) {
                        Some(any_listen_entry) => any_listen_entry,
                        None => {
                            tracing::error!(
                                "Tried to demux with a missing session and no listen bindings"
                            );
                            return Err(DemuxError::MissingSession);
                        }
                    },
                };
                let socket = match self.sockets.entry(*binding) {
                    Entry::Occupied(sock) => sock.get().clone(),
                    Entry::Vacant(_) => {
                        return Err(DemuxError::MissingSession);
                    }
                };
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(None),
                    downstream: caller,
                    stored_messages: RwLock::new(VecDeque::new()),
                });
                session.stored_messages.write().unwrap().push_back(message);
                socket.add_listen_address(identifier.remote);
                entry.insert(session);
            }
        };
        Ok(())
    }

    fn notify(&self, notification: NotifyType, caller: Arc<dyn Session>, control: Control) {
        match notification {
            NotifyType::NewConnection => {
                let identifier = match control.get::<UdpHeader>() {
                    Some(udp_header) => {
                        let ipv4_header = control.get::<Ipv4Header>().unwrap();
                        Endpoints::new(
                            Endpoint::new(ipv4_header.destination, udp_header.destination),
                            Endpoint::new(ipv4_header.source, udp_header.source),
                        )
                    }
                    None => match control.get::<Endpoints>() {
                        Some(endpoints) => *endpoints,
                        None => return,
                    },
                };
                let any_identifier =
                    Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local.port);
                match self.socket_sessions.entry(identifier) {
                    Entry::Occupied(entry) => entry.get().clone().connection_established(),
                    Entry::Vacant(entry) => {
                        // If the session does not exist, see if we have a listen
                        // binding for it
                        let binding = match self.listen_bindings.get(&identifier.local) {
                            Some(listen_entry) => listen_entry,
                            // If we don't have a normal listen binding, check for
                            // a 0.0.0.0 binding
                            None => match self.listen_bindings.get(&any_identifier) {
                                Some(any_listen_entry) => any_listen_entry,
                                None => {
                                    return;
                                }
                            },
                        };
                        let socket = match self.sockets.entry(*binding) {
                            Entry::Occupied(sock) => sock.get().clone(),
                            Entry::Vacant(_) => {
                                return;
                            }
                        };
                        let session = Arc::new(SocketSession {
                            upstream: RwLock::new(None),
                            downstream: caller,
                            stored_messages: RwLock::new(VecDeque::new()),
                        });
                        socket.add_listen_address(identifier.remote);
                        entry.insert(session);
                    }
                };
            }
            NotifyType::NewMessage => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("There is already a session for {0:?}")]
    Existing(Endpoints),
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
    #[error("There was no socket for the file descriptor {0}")]
    NoSocketForFd(u64),
}
