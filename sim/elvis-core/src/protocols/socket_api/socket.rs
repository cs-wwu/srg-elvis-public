use super::SocketAPI;
use crate::{
    message::Chunk,
    protocols::{
        dns::dns_resolver::DnsResolver,
        utility::{Endpoint, Endpoints},
    },
    Machine, Message, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use thiserror::Error as ThisError;
use tokio::{select, sync::mpsc::Receiver, task::yield_now};

/// An implementation of an individual Socket
/// Created by the [`SocketAPI`]
pub struct Socket {
    family: ProtocolFamily,
    sock_type: SocketType,
    pub(crate) is_active: bool,
    is_bound: bool,
    pub(crate) is_listening: bool,
    is_blocking: bool,
    pub(crate) local_addr: Option<Endpoint>,
    pub(crate) remote_addr: Option<Endpoint>,
    session: RwLock<Option<Arc<dyn Session>>>,
    listen_backlog: usize,
    machine: Arc<Machine>,
    socket_api: Arc<SocketAPI>,
    message_receiver: Option<Receiver<Message>>,
    stored_message: Option<Message>,
    pub(crate) connection_receiver: Option<Receiver<Endpoint>>,
    shutdown: Shutdown,
}

impl Socket {
    pub(super) fn new(
        domain: ProtocolFamily,
        sock_type: SocketType,
        machine: Arc<Machine>,
        socket_api: Arc<SocketAPI>,
        shutdown: Shutdown,
    ) -> Socket {
        Self {
            family: domain,
            sock_type,
            is_active: false,
            is_bound: false,
            is_listening: false,
            is_blocking: true,
            local_addr: None,
            remote_addr: None,
            listen_backlog: 0,
            session: Default::default(),
            machine,
            socket_api,
            message_receiver: Default::default(),
            stored_message: None,
            connection_receiver: Default::default(),
            shutdown,
        }
    }

    /// Used to specify whether or not certain socket functions should block
    pub fn set_blocking(&mut self, is_blocking: bool) -> &mut Self {
        self.is_blocking = is_blocking;
        self
    }

    /// TODO(HenryEricksonIV) Used by calling application when the ip address
    /// of the endpoint is not known to the calling application.
    /// Intended to call 'connect()' with an ip provided by the local
    /// 'DnsResolver'.
    pub async fn connect_by_name(
        &mut self,
        domain_name: String,
        dest_port: u16,
    ) -> Result<&mut Self, SocketError> {
        let ip_from_domain = self
            .machine
            .protocol::<DnsResolver>()
            .unwrap()
            .get_host_by_name(domain_name, self.machine.clone())
            .await
            .unwrap();
        let new_destination = Endpoint::new(ip_from_domain, dest_port);
        self.connect(new_destination).await
    }

    /// Assigns a remote ip address and port to a socket and connects the socket
    /// to that endpoint
    pub async fn connect(&mut self, sock_addr: Endpoint) -> Result<&mut Self, SocketError> {
        // A socket can only be connected once, subsequent calls to connect will
        // throw an error if the socket is already connected. Also, a listening
        // socket cannot connect to a remote endpoint
        if self.is_active || self.is_listening {
            return Err(SocketError::AcceptError);
        }
        if self.local_addr.is_none() {
            self.local_addr = Some(self.socket_api.get_ephemeral_endpoint().unwrap());
        }
        // Assign the given remote socket address to the socket
        self.remote_addr = Some(sock_addr);
        // Gather the necessary data to open a session and pass it on to the
        // Sockets API to retreive a socket_session
        let local_op = self.local_addr;
        let remote_op = self.remote_addr;
        if let (Some(local), Some(remote)) = (local_op, remote_op) {
            let (session, receiver) = match self
                .machine
                .protocol::<SocketAPI>()
                .expect("Sockets API not found")
                .open(
                    Endpoints::new(local, remote),
                    self.sock_type,
                    self.machine.clone(),
                )
                .await
            {
                Ok(v) => v,
                Err(_) => return Err(SocketError::ConnectError),
            };
            // Assign the socket_session to the socket
            self.message_receiver = Some(receiver);
            *self.session.write().unwrap() = Some(session);
            self.is_active = true;
            Ok(self)
        } else {
            Err(SocketError::ConnectError)
        }
    }

    /// Assigns a local ip address and port to a socket
    pub fn bind(&mut self, sock_addr: Endpoint) -> Result<&mut Self, SocketError> {
        match self.family {
            ProtocolFamily::LOCAL => {
                return Err(SocketError::BindError);
            }
            ProtocolFamily::INET => self.local_addr = Some(sock_addr),
            ProtocolFamily::INET6 => return Err(SocketError::BindError),
        }
        self.is_bound = true;
        Ok(self)
    }

    /// Makes this socket a listening socket, meaning that it can no longer be
    /// used to send or receive messages, but can instead be used to accept
    /// incoming connections on the specified port via accept()
    pub fn listen(&mut self, backlog: usize) -> Result<&mut Self, SocketError> {
        if !self.is_bound || self.is_active || self.is_listening {
            return Err(SocketError::AcceptError);
        }

        if let Some(local_addr) = self.local_addr {
            match self
                .machine
                .protocol::<SocketAPI>()
                .expect("Sockets API not found")
                .listen(local_addr, self.sock_type, backlog, self.machine.clone())
            {
                Ok(receiver) => {
                    self.is_listening = true;
                    self.listen_backlog = backlog;
                    self.connection_receiver = Some(receiver);
                    Ok(self)
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
    pub async fn accept(&mut self) -> Result<Socket, SocketError> {
        if !self.is_listening || self.is_active {
            return Err(SocketError::AcceptError);
        }
        let mut shutdown_receiver = self.shutdown.receiver();
        let connection_receiver = match &mut self.connection_receiver {
            Some(v) => v,
            None => return Err(SocketError::AcceptError),
        };
        let endpoint = select! {
            _ = shutdown_receiver.recv() => { return Err(SocketError::Shutdown) },
            endpoint = connection_receiver.recv() => endpoint,
        };
        let mut new_sock = self
            .socket_api
            .new_socket(self.family, self.sock_type, self.machine.clone())
            .await?;
        let local_addr = Endpoint {
            address: self.socket_api.get_local_ip()?,
            port: self.local_addr.unwrap().port,
        };
        new_sock.bind(local_addr)?;
        new_sock.remote_addr = endpoint;
        let (session, receiver) = self
            .socket_api
            .get_socket_session(new_sock.local_addr.unwrap(), new_sock.remote_addr.unwrap())?;
        new_sock.message_receiver = Some(receiver);
        *new_sock.session.write().unwrap() = Some(session.clone());
        new_sock.is_active = true;
        session.receive_stored_messages().unwrap();
        Ok(new_sock)
    }

    /// Sends data to the socket's remote endpoint
    pub fn send(
        &self,
        message: impl Into<Chunk> + std::marker::Send + 'static,
    ) -> Result<(), SocketError> {
        if self.session.read().unwrap().is_none() || self.is_listening {
            return Err(SocketError::SendError);
        }
        let session = self.session.read().unwrap().as_ref().unwrap().clone();
        let protocols = self.machine.clone();
        tokio::spawn(async move {
            session.send(Message::new(message), protocols).unwrap();
        });
        Ok(())
    }

    /// Receives data from the socket's remote endpoint
    ///
    /// This function will block if the queue of incoming messages is empty
    /// until a new message is received
    pub async fn recv(&mut self, bytes: usize) -> Result<Vec<u8>, SocketError> {
        // If the socket doesn't have a session yet, data cannot be received and
        // calls to recv will return an error, a call to connect() must be made
        // first
        if self.session.read().unwrap().is_none() || self.is_listening {
            return Err(SocketError::ReceiveError);
        }
        yield_now().await;
        let mut shutdown_receiver = self.shutdown.receiver();
        let message_receiver = match &mut self.message_receiver {
            Some(v) => v,
            None => return Err(SocketError::AcceptError),
        };
        let mut buf = Vec::new();
        // If the socket still has a portion of a message stored, read from that first
        if let Some(mut message) = self.stored_message.take() {
            if message.len() <= bytes {
                buf.extend(message.iter());
            } else {
                buf.extend(message.iter().take(bytes));
                message.slice(bytes..);
                self.stored_message = Some(message);
            }
        }
        // Then start receiving more bytes from the socket's receiver
        while buf.len() < bytes {
            let mut message = if buf.is_empty() && self.is_blocking {
                select! {
                    _ = shutdown_receiver.recv() => { return Err(SocketError::Shutdown); },
                    message = message_receiver.recv() => {
                        match message {
                            Some(msg) => msg,
                            None => return Err(SocketError::ReceiveError),
                        }
                    },
                }
            } else {
                match message_receiver.try_recv() {
                    Ok(msg) => msg,
                    Err(_) => {
                        break;
                    }
                }
            };
            if message.len() <= bytes {
                buf.extend(message.iter());
            } else {
                buf.extend(message.iter().take(bytes));
                message.slice(bytes..);
                self.stored_message = Some(message);
            }
        }
        Ok(buf)
    }

    /// Receives a [`Message`] from the socket's remote endpoint
    ///
    /// This function will block if the queue of incoming messages is empty
    /// until a new message is received
    pub async fn recv_msg(&mut self) -> Result<Message, SocketError> {
        // If the socket doesn't have a session yet, data cannot be received and
        // calls to recv will return an error, a call to connect() must be made
        // first
        if self.session.read().unwrap().is_none() || self.is_listening {
            return Err(SocketError::ReceiveError);
        }
        yield_now().await;
        match self.stored_message.take() {
            Some(msg) => Ok(msg),
            None => {
                let mut shutdown_receiver = self.shutdown.receiver();
                let message_receiver = match &mut self.message_receiver {
                    Some(v) => v,
                    None => return Err(SocketError::ReceiveError),
                };
                if self.is_blocking {
                    select! {
                        _ = shutdown_receiver.recv() => Err(SocketError::Shutdown),
                        message = message_receiver.recv() => {
                            match message {
                                Some(msg) => Ok(msg),
                                None => Err(SocketError::ReceiveError),
                            }
                        },
                    }
                } else {
                    match message_receiver.try_recv() {
                        Ok(msg) => Ok(msg),
                        Err(_) => Err(SocketError::ReceiveError),
                    }
                }
            }
        }
    }

    pub fn close(self) {
        if let Some(socket_api) = self.machine.protocol::<SocketAPI>() {
            socket_api.close_and_drop_socket(self);
        }
    }

    fn close_during_drop(&mut self) {
        if let Some(socket_api) = self.machine.protocol::<SocketAPI>() {
            socket_api.close_socket(self);
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close_during_drop();
    }
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
