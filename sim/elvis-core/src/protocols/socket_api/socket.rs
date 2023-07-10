use super::SocketAPI;
use crate::{
    machine::ProtocolMap,
    message::Chunk,
    protocol::{DemuxError, NotifyType},
    protocols::{utility::{Endpoint, Endpoints}, dns::dns_client::DnsClient},
    Message, Session, Shutdown,
};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;
use tokio::{select, sync::Notify};

/// An implementation of an individual Socket
/// Created by the [`Sockets`] API
pub struct Socket {
    pub family: ProtocolFamily,
    pub sock_type: SocketType,
    fd: u64,
    is_active: RwLock<bool>,
    is_bound: RwLock<bool>,
    is_listening: RwLock<bool>,
    is_blocking: RwLock<bool>,
    local_addr: RwLock<Option<Endpoint>>,
    remote_addr: RwLock<Option<Endpoint>>,
    session: Arc<RwLock<Option<Arc<dyn Session>>>>,
    listen_addresses: Arc<RwLock<VecDeque<Endpoint>>>,
    listen_backlog: RwLock<usize>,
    notify_listen: Notify,
    messages: Arc<RwLock<VecDeque<Message>>>,
    notify_recv: Notify,
    protocols: ProtocolMap,
    socket_api: Arc<SocketAPI>,
    shutdown: Shutdown,
}

impl Socket {
    pub(super) fn new(
        domain: ProtocolFamily,
        sock_type: SocketType,
        fd: u64,
        protocols: ProtocolMap,
        socket_api: Arc<SocketAPI>,
        shutdown: Shutdown,
    ) -> Socket {
        Self {
            family: domain,
            sock_type,
            fd,
            is_active: RwLock::new(false),
            is_bound: RwLock::new(false),
            is_listening: RwLock::new(false),
            is_blocking: RwLock::new(true),
            local_addr: RwLock::new(None),
            remote_addr: RwLock::new(None),
            listen_addresses: Default::default(),
            listen_backlog: RwLock::new(0),
            notify_listen: Notify::new(),
            messages: Default::default(),
            notify_recv: Notify::new(),
            session: Default::default(),
            protocols,
            socket_api,
            shutdown,
        }
    }

    pub(super) fn add_listen_address(&self, remote_address: Endpoint) {
        let backlog = *self.listen_backlog.read().unwrap();
        if backlog == 0 || self.listen_addresses.read().unwrap().len() <= backlog {
            self.listen_addresses
                .write()
                .unwrap()
                .push_back(remote_address);
            self.notify_listen.notify_one();
        }
    }

    async fn wait_for_notify(&self, notify_type: NotifyType) -> NotifyResult {
        if *self.is_blocking.read().unwrap() {
            let mut shutdown_receiver = self.shutdown.receiver();
            println!("wait_for_notify");
            match notify_type {
                NotifyType::NewConnection => select! {
                    _ = shutdown_receiver.recv() => NotifyResult::Shutdown,
                    _ = self.notify_listen.notified() => NotifyResult::Notified,
                },
                NotifyType::NewMessage => select! {
                    _ = shutdown_receiver.recv() => NotifyResult::Shutdown,
                    _ = self.notify_recv.notified() => NotifyResult::Notified,
                },
            }
        } else {
            NotifyResult::Notified
        }
    }

    /// Used to specify whether or not certain socket functions should block
    pub fn set_blocking(&self, is_blocking: bool) {
        *self.is_blocking.write().unwrap() = is_blocking;
    }

    pub fn connection_established(&self) {
        self.notify_listen.notify_one();
    }

    /// TODO(HenryEricksonIV) Used by calling application when the ip address
    /// of the endpoint is not known to the calling application.
    /// Intended to call 'connect()' with an ip provided by the local 
    /// 'DnsClient'.
    pub async fn connect_by_name(&self, domain_name: String, dest_port: u16) -> Result<(), SocketError> {
        println!("connect by name");
        let ip_from_domain = self.protocols.protocol::<DnsClient>()
            .unwrap()
            .get_host_by_name(domain_name, self.protocols.clone()).await
            .unwrap();
        let new_destination = Endpoint::new(ip_from_domain, dest_port);
        self.connect(new_destination).await
    }

    /// Assigns a remote ip address and port to a socket and connects the socket
    /// to that endpoint
    pub async fn connect(&self, sock_addr: Endpoint) -> Result<(), SocketError> {
        // A socket can only be connected once, subsequent calls to connect will
        // throw an error if the socket is already connected. Also, a listening
        // socket cannot connect to a remote endpoint
        println!("connect");
        println!(" first boolean {:?}", self.is_active.read().unwrap());
        println!(" second boolean {:?}", self.is_listening.read().unwrap());
        if *self.is_active.read().unwrap() || *self.is_listening.read().unwrap() {
            return Err(SocketError::AcceptError);
        }
        if self.local_addr.read().unwrap().is_none() {
            *self.local_addr.write().unwrap() =
                Some(self.socket_api.get_ephemeral_endpoint().unwrap());
        }
        // Assign the given remote socket address to the socket
        *self.remote_addr.write().unwrap() = Some(sock_addr);
        // Gather the necessary data to open a session and pass it on to the
        // Sockets API to retreive a socket_session
        let local_op = *self.local_addr.read().unwrap();
        let remote_op = *self.remote_addr.read().unwrap();
        println!("connect 2");
        println!("local: {:?}\nremote: {:?}", local_op, remote_op);
        if let (Some(local), Some(remote)) = (local_op, remote_op) {
            let session = match self
            .protocols
            .protocol::<SocketAPI>()
            .expect("Sockets API not found")
            .open_with_fd(
                self.fd,
                Endpoints::new(local, remote),
                self.protocols.clone(),
            )
            .await
            {
                Ok(v) => v,
                Err(_) => return Err(SocketError::ConnectError),
            };
            println!("connect 3");
            // Assign the socket_session to the socket
            *self.session.write().unwrap() = Some(session);
            *self.is_active.write().unwrap() = true;
            Ok(())
        } else {
            Err(SocketError::ConnectError)
        }
    }

    /// Assigns a local ip address and port to a socket
    pub fn bind(&self, sock_addr: Endpoint) -> Result<(), SocketError> {
        match self.family {
            ProtocolFamily::LOCAL => {
                return Err(SocketError::BindError);
            }
            ProtocolFamily::INET => *self.local_addr.write().unwrap() = Some(sock_addr),
            ProtocolFamily::INET6 => return Err(SocketError::BindError),
        }
        *self.is_bound.write().unwrap() = true;
        Ok(())
    }

    /// Makes this socket a listening socket, meaning that it can no longer be
    /// used to send or receive messages, but can instead be used to accept
    /// incoming connections on the specified port via accept()
    pub fn listen(&self, backlog: usize) -> Result<(), SocketError> {
        if !*self.is_bound.read().unwrap()
            || *self.is_active.read().unwrap()
            || *self.is_listening.read().unwrap()
        {
            return Err(SocketError::AcceptError);
        }

        if let Some(local_addr) = *self.local_addr.read().unwrap() {
            match self
                .protocols
                .protocol::<SocketAPI>()
                .expect("Sockets API not found")
                .listen_with_fd(self.fd, local_addr, self.protocols.clone())
            {
                Ok(_) => {
                    *self.is_listening.write().unwrap() = true;
                    *self.listen_backlog.write().unwrap() = backlog;
                    Ok(())
                }
                Err(_) => Err(SocketError::ListenError),
            }
        } else {
            Err(SocketError::ListenError)
        }
    }

    /// Takes the first connection out of this socket's queue of pending
    /// connections, assigns it to a new socket, and returns the new socket
    ///
    /// This function will block if the queue of pending connections is empty
    /// until a new connection arrives
    pub async fn accept(&self) -> Result<Arc<Socket>, SocketError> {
        println!("accept");
        if !*self.is_listening.read().unwrap() || *self.is_active.read().unwrap() {
            return Err(SocketError::AcceptError);
        }
        if self.wait_for_notify(NotifyType::NewConnection).await == NotifyResult::Shutdown {
            println!("accept wait_for_notify returned shutdown error");
            return Err(SocketError::Shutdown);
        }
        println!("made it to accept");
        let new_sock = self
        .socket_api
        .new_socket(self.family, self.sock_type, self.protocols.clone())
        .await?;
        println!("accept 2");
        let local_addr = Endpoint {
            address: self.socket_api.get_local_ip()?,
            port: self.local_addr.read().unwrap().unwrap().port,
        };
        new_sock.bind(local_addr)?;
        *new_sock.remote_addr.write().unwrap() = self.listen_addresses.write().unwrap().pop_front();
        if !self.listen_addresses.read().unwrap().is_empty() {
            self.notify_listen.notify_one();
        }
        let session = self.socket_api.get_socket_session(
            new_sock.local_addr.read().unwrap().unwrap(),
            new_sock.remote_addr.read().unwrap().unwrap(),
        )?;
        *session.upstream.write().unwrap() = Some(new_sock.clone());
        *new_sock.session.write().unwrap() = Some(session.clone());
        session.receive_stored_messages().unwrap();
        *new_sock.is_active.write().unwrap() = true;
        Ok(new_sock)
    }

    /// Sends data to the socket's remote endpoint
    pub fn send(
        &self,
        message: impl Into<Chunk> + std::marker::Send + 'static,
    ) -> Result<(), SocketError> {
        println!(" first boolean {:?}", self.session.read().unwrap().is_none());
        println!(" second boolean {:?}", self.is_listening.read());
        if self.session.read().unwrap().is_none() || *self.is_listening.read().unwrap() {
            return Err(SocketError::SendError);
        }
        println!("socket send");
        let session = self.session.clone();
        let protocols = self.protocols.clone();
        tokio::spawn(async move {
            session
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .send(Message::new(message), protocols)
                .unwrap();
        });
        Ok(())
    }

    /// Receives data from the socket's remote endpoint
    ///
    /// This function will block if the queue of incoming messages is empty
    /// until a new message is received
    pub async fn recv(&self, bytes: usize) -> Result<Vec<u8>, SocketError> {
        // If the socket doesn't have a session yet, data cannot be received and
        // calls to recv will return an error, a call to connect() must be made
        // first
        if self.session.read().unwrap().is_none() || *self.is_listening.read().unwrap() {
            return Err(SocketError::ReceiveError);
        }
        // If there is no data in the queue to recv, and the socket is blocking,
        // block until there is data to be received
        println!("socket recv");
        if self.wait_for_notify(NotifyType::NewMessage).await == NotifyResult::Shutdown {
            return Err(SocketError::Shutdown);
        }
        let mut buf = Vec::new();
        let queue = &mut *self.messages.write().unwrap();
        while let Some(text) = queue.front_mut() {
            if text.len() <= bytes {
                buf.extend(text.iter());
                queue.pop_front();
            } else {
                buf.extend(text.iter().take(bytes));
                text.slice(bytes..);
                break;
            }
        }
        if !queue.is_empty() {
            self.notify_recv.notify_one();
        }
        Ok(buf)
    }

    /// Receives a [`Message`] from the socket's remote endpoint
    ///
    /// This function will block if the queue of incoming messages is empty
    /// until a new message is received
    pub async fn recv_msg(&self) -> Result<Message, SocketError> {
        // If the socket doesn't have a session yet, data cannot be received and
        // calls to recv will return an error, a call to connect() must be made
        // first
        if self.session.read().unwrap().is_none() || *self.is_listening.read().unwrap() {
            return Err(SocketError::ReceiveError);
        }
        // If there is no data in the queue to recv, and the socket is blocking,
        // block until there is data to be received
        if self.wait_for_notify(NotifyType::NewMessage).await == NotifyResult::Shutdown {
            println!("recv_msg wait_for_notify returned shutdown error");
            return Err(SocketError::Shutdown);
        }
        let mut queue = self.messages.write().unwrap().clone();
        let msg = match queue.pop_front() {
            Some(v) => v,
            None => return Err(SocketError::Other),
        };
        if !queue.is_empty() {
            self.notify_recv.notify_one();
        }
        Ok(msg)
    }

    /// Called by the socket's socket_session when it receives data, stores data
    /// in a queue, which is emptied by calls to recv() or recv_msg()
    pub(crate) fn receive(&self, message: Message) -> Result<(), DemuxError> {
        self.messages.write().unwrap().push_back(message);
        self.notify_recv.notify_one();
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NotifyResult {
    Notified,
    Shutdown,
}

#[derive(Debug, ThisError, Clone, PartialEq, Eq)]
pub enum SocketError {
    #[error("Bind error")]
    BindError,
    #[error("Connect error")]
    ConnectError,
    #[error("Listen error")]
    ListenError,
    #[error("Accept error")]
    AcceptError,
    #[error("Send error")]
    SendError,
    #[error("Receive error")]
    ReceiveError,
    #[error("The simulation requested to shut down")]
    Shutdown,
    #[error("Unspecified error")]
    Other,
}

/// ProtocolFamily::LOCAL - Indicates that the socket is to be used to
/// communicate with other applications on the same machine
/// (Not yet implemented)
///
/// ProtocolFamily::INET - Indicates that the socket utilizes IPv4
///
/// ProtocolFamily::INET6 - Indicates that the socket utilizes IPv6
/// (Not yet implemented)
#[derive(Clone, Copy)]
pub enum ProtocolFamily {
    LOCAL,
    INET,
    INET6,
}

/// SocketType::Stream - Indicates that the socket utilizes TCP
///
/// SocketType::Datagram - Indicates that the socket utilizes UDP
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SocketType {
    Stream,
    Datagram,
}
