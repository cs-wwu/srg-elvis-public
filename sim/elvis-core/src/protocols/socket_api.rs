use super::{
    ipv4::ipv4_parsing::Ipv4Header,
    tcp,
    udp::{self, UdpHeader},
    utility::Endpoint,
    Endpoints, Tcp,
};
use crate::{
    protocol::{DemuxError, NotifyType, StartError},
    protocols::{ipv4::Ipv4Address, Udp},
    Control, FxDashMap, Machine, Message, Protocol, Session, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{
    any::TypeId,
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Barrier, Notify,
};

pub mod socket;
use socket::{ProtocolFamily, Socket, SocketError, SocketType};

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
pub struct SocketAPI {
    local_address: Option<Ipv4Address>,
    local_ports: RwLock<u16>,
    // This type seems absurd, but its the best way I've found to achieve the required behavior.
    // Most functions can freely write to the DashMap concurrently, and will use a read lock.
    // get_socket_session requires exclusive write permissions, and will use a write lock.
    socket_sessions: RwLock<FxDashMap<Endpoints, Arc<SocketSession>>>,
    listen_bindings: FxDashMap<Endpoint, Sender<Endpoint>>,
    notify_init: Notify,
    shutdown: RwLock<Option<Shutdown>>,
    is_shutdown: RwLock<bool>,
}

impl SocketAPI {
    /// Creates a new instance of the protocol
    pub fn new(local_address: Option<Ipv4Address>) -> Self {
        Self {
            local_address,
            local_ports: RwLock::new(49152),
            socket_sessions: Default::default(),
            listen_bindings: Default::default(),
            notify_init: Notify::new(),
            shutdown: Default::default(),
            is_shutdown: RwLock::new(false),
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
        machine: Arc<Machine>,
    ) -> Result<Socket, SocketError> {
        self.notify_init.notified().await;
        let socket = Socket::new(
            domain,
            sock_type,
            machine,
            self.clone(),
            self.shutdown.read().unwrap().as_ref().unwrap().clone(),
        );
        self.notify_init.notify_one();
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
    ) -> Result<(Arc<SocketSession>, Receiver<Message>), SocketError> {
        let listen_local = Endpoint::new(self.local_address.unwrap(), local.port);
        let listen_identifier = Endpoints::new(listen_local, remote);
        let identifier = Endpoints::new(local, remote);

        let session_map = self.socket_sessions.write().unwrap();
        let session = match session_map.entry(listen_identifier) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => match *self.is_shutdown.read().unwrap() {
                true => {
                    return Err(SocketError::Shutdown);
                }
                false => {
                    return Err(SocketError::AcceptError);
                }
            },
        };
        session_map.insert(identifier, session.clone());

        let (sender, receiver) = mpsc::channel(u8::MAX.into());
        *session.upstream.write().unwrap() = Some(sender);
        Ok((session, receiver))
    }

    /// Called from Socket::connect()
    /// Creates a new socket_session based on IP address and port and returns it
    pub async fn open(
        &self,
        socket_id: Endpoints,
        transport: SocketType,
        machine: Arc<Machine>,
    ) -> Result<(Arc<dyn Session>, Receiver<Message>), OpenError> {
        let downstream = match transport {
            SocketType::Datagram => {
                machine
                    .protocol::<Udp>()
                    .unwrap()
                    .open_and_listen(TypeId::of::<Self>(), socket_id, machine)
                    .await?
            }
            SocketType::Stream => {
                machine
                    .protocol::<Tcp>()
                    .unwrap()
                    .open(TypeId::of::<Self>(), socket_id, machine)
                    .await?
            }
        };
        match self.socket_sessions.read().unwrap().entry(socket_id) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing(socket_id))?
            }
            Entry::Vacant(entry) => {
                let (sender, receiver) = mpsc::channel(u8::MAX.into());
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(Some(sender)),
                    downstream,
                    stored_messages: RwLock::new(VecDeque::new()),
                });
                entry.insert(session.clone());
                Ok((session, receiver))
            }
        }
    }

    fn listen(
        &self,
        address: Endpoint,
        transport: SocketType,
        backlog: usize,
        machine: Arc<Machine>,
    ) -> Result<Receiver<Endpoint>, ListenError> {
        if self.listen_bindings.get(&address).is_some() {
            return Err(ListenError::Existing(address));
        }
        let (sender, receiver) = mpsc::channel(backlog);
        self.listen_bindings.insert(address, sender);
        match transport {
            SocketType::Datagram => machine
                .protocol::<Udp>()
                .expect("No such protocol")
                .listen(TypeId::of::<Self>(), address, machine)?,
            SocketType::Stream => machine
                .protocol::<Tcp>()
                .expect("No such protocol")
                .listen(TypeId::of::<Self>(), address, machine)?,
        };
        Ok(receiver)
    }

    fn close_and_drop_socket(&self, mut socket: Socket) {
        self.close_socket(&mut socket);
        drop(socket);
    }

    fn close_socket(&self, socket: &mut Socket) {
        if socket.is_active {
            if let (Some(local_addr), Some(remote_addr)) = (socket.local_addr, socket.remote_addr) {
                let identifier = Endpoints::new(local_addr, remote_addr);
                self.socket_sessions.read().unwrap().remove(&identifier);
            }
        }
        if socket.is_listening {
            if let Some(local_addr) = socket.local_addr {
                self.listen_bindings.remove(&local_addr);
                if let Some(ref mut receiver) = socket.connection_receiver {
                    while let Ok(remote_endpoint) = receiver.try_recv() {
                        let identifier = Endpoints::new(local_addr, remote_endpoint);
                        self.socket_sessions.read().unwrap().remove(&identifier);
                    }
                }
            }
        }
    }

    fn shutdown(&self) {
        *self.is_shutdown.write().unwrap() = true;
        self.socket_sessions.write().unwrap().clear();
    }
}

impl Protocol for SocketAPI {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        // Listen on the local IP address.
        // This is necessary for ARP purposes.
        // A machine must set its local IP so it can tell other machines about itself.
        // TODO(sudobeans): SocketAPI shouldn't need to know about ARP!
        // Seems like code smell (on ARP's part).
        if let Some(local_address) = self.local_address {
            if let Some(arp) = machine.protocol::<crate::protocols::Arp>() {
                arp.listen(local_address);
            }
        }

        *self.shutdown.write().unwrap() = Some(shutdown.clone());
        self.notify_init.notify_one();
        initialized.wait().await;
        let self_handle = machine.protocol::<SocketAPI>().unwrap();
        tokio::spawn(async move {
            _ = shutdown.receiver().recv().await;
            self_handle.shutdown();
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
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        let identifier = match control.get::<Endpoints>() {
            Some(endpoints) => *endpoints,
            None => Endpoints::new_from_headers(
                control.get::<UdpHeader>(),
                control.get::<Ipv4Header>(),
            )?,
        };
        let any_identifier = Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local.port);

        match self.socket_sessions.read().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().receive(message)?,
            Entry::Vacant(entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let sender = match self.listen_bindings.get(&identifier.local) {
                    Some(listen_entry) => listen_entry,
                    None => match self.listen_bindings.get(&any_identifier) {
                        Some(any_listen_entry) => any_listen_entry,
                        None => {
                            return Err(DemuxError::MissingSession);
                        }
                    },
                };
                let session = Arc::new(SocketSession {
                    upstream: RwLock::new(None),
                    downstream: caller,
                    stored_messages: RwLock::new(VecDeque::new()),
                });
                session.stored_messages.write().unwrap().push_back(message);
                match sender.try_send(identifier.remote) {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(DemuxError::MissingSession);
                    }
                };
                entry.insert(session);
            }
        };
        Ok(())
    }

    fn notify(&self, notification: NotifyType, caller: Arc<dyn Session>, control: Control) {
        match notification {
            NotifyType::NewConnection => {
                let identifier = match control.get::<Endpoints>() {
                    Some(endpoints) => *endpoints,
                    None => match Endpoints::new_from_headers(
                        control.get::<UdpHeader>(),
                        control.get::<Ipv4Header>(),
                    ) {
                        Ok(endpoints) => endpoints,
                        Err(_) => {
                            return;
                        }
                    },
                };
                let any_identifier =
                    Endpoint::new(Ipv4Address::CURRENT_NETWORK, identifier.local.port);
                match self.socket_sessions.read().unwrap().entry(identifier) {
                    Entry::Occupied(_) => {}
                    Entry::Vacant(entry) => {
                        // If the session does not exist, see if we have a listen
                        // binding for it
                        let sender = match self.listen_bindings.get(&identifier.local) {
                            Some(listen_entry) => listen_entry,
                            None => match self.listen_bindings.get(&any_identifier) {
                                Some(any_listen_entry) => any_listen_entry,
                                None => {
                                    return;
                                }
                            },
                        };
                        let session = Arc::new(SocketSession {
                            upstream: RwLock::new(None),
                            downstream: caller,
                            stored_messages: RwLock::new(VecDeque::new()),
                        });
                        match sender.try_send(identifier.remote) {
                            Ok(_) => {}
                            Err(e) => println!("Notify Error: {:?}", e),
                        }
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
