use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError, ListenError, NotifyType, OpenError, QueryError, StartError},
    protocols::{ipv4::Ipv4Address, Ipv4, Tcp, Udp},
    session::SharedSession,
    Control, FxDashMap, Id, Message, Protocol, ProtocolMap, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use tokio::sync::{Barrier, Notify};

pub mod socket;
use socket::{ProtocolFamily, Socket, SocketError, SocketId, SocketType};

mod socket_session;
use socket_session::SocketSession;

use super::utility::Endpoint;

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
    local_address: Option<Ipv4Address>,
    local_ports: RwLock<u16>,
    fds: RwLock<u64>,
    sockets: Arc<FxDashMap<Id, Arc<Socket>>>,
    socket_sessions: FxDashMap<SocketId, Arc<SocketSession>>,
    listen_bindings: FxDashMap<Endpoint, Id>,
    notify_init: Notify,
    shutdown: RwLock<Option<Shutdown>>,
}

impl Sockets {
    /// A unique identifier for the protocol
    pub const ID: Id = Id::from_string("Sockets");

    /// Creates a new instance of the protocol
    pub fn new(local_address: Option<Ipv4Address>) -> Self {
        Self {
            local_address,
            local_ports: RwLock::new(49152),
            fds: RwLock::new(0),
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
        let sock = match self.sockets.entry(upstream) {
            Entry::Occupied(sock) => sock.get().clone(),
            Entry::Vacant(_) => return Err(OpenError::MissingContext),
        };
        let identifier = SocketId::new(
            Ipv4::get_local_address(&participants).map_err(|_| {
                tracing::error!("Missing local address on context");
                OpenError::MissingContext
            })?,
            match sock.sock_type {
                SocketType::Datagram => Udp::get_local_port(&participants),
                SocketType::Stream => Tcp::get_local_port(&participants),
            }
            .map_err(|_| {
                tracing::error!("Missing local port on context");
                OpenError::MissingContext
            })?,
            Ipv4::get_remote_address(&participants).map_err(|_| {
                tracing::error!("Missing remote address on context");
                OpenError::MissingContext
            })?,
            match sock.sock_type {
                SocketType::Datagram => Udp::get_remote_port(&participants),
                SocketType::Stream => Tcp::get_remote_port(&participants),
            }
            .map_err(|_| {
                tracing::error!("Missing local port on context");
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
                    .protocol(match sock.sock_type {
                        SocketType::Datagram => Udp::ID,
                        SocketType::Stream => Tcp::ID,
                    })
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols)?;
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

    fn listen(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        let sock = match self.sockets.entry(upstream) {
            Entry::Occupied(sock) => sock.get().clone(),
            Entry::Vacant(_) => return Err(ListenError::MissingContext),
        };
        let identifier = Endpoint::new(
            Ipv4::get_local_address(&participants).map_err(|_| {
                tracing::error!("Missing local address on context");
                ListenError::MissingContext
            })?,
            match sock.sock_type {
                SocketType::Datagram => Udp::get_local_port(&participants),
                SocketType::Stream => Tcp::get_local_port(&participants),
            }
            .map_err(|_| {
                tracing::error!("Missing local port on context");
                ListenError::MissingContext
            })?,
        );
        self.listen_bindings.insert(identifier, upstream);
        protocols
            .protocol(match sock.sock_type {
                SocketType::Datagram => Udp::ID,
                SocketType::Stream => Tcp::ID,
            })
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
        let identifier = SocketId::new_from_context(context)?;
        let any_identifier =
            Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local_address.port);
        match self.socket_sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone().receive(message)?,
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
                            return Err(DemuxError::MissingSession)?;
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
                socket.add_listen_address(identifier.remote_address);
                entry.insert(session.clone());
                //session
            }
        };
        //session.receive(message)?;
        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }

    fn notify(&self, notification: NotifyType, caller: SharedSession, context: Context) {
        let Ok(identifier) = (match notification {
            NotifyType::NewConnection => SocketId::new_from_context(context),
            NotifyType::NewMessage => Err(DemuxError::Other)
        }) else { return };
        let any_identifier =
            Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local_address.port);
        match self.socket_sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone().connection_established(),
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
                socket.add_listen_address(identifier.remote_address);
                entry.insert(session.clone());
            }
        };
    }
}
