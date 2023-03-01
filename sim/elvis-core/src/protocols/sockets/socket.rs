use std::{sync::{RwLock, Arc}, collections::VecDeque};

use thiserror::Error as ThisError;
use tokio::sync::Notify;
use crate::{
    protocols::{
        ipv4::Ipv4Address,
        Ipv4,
        Udp
    },
    session::SharedSession,
    Control,
    ProtocolMap,
    Id,
    message::Chunk,
    Message,
    protocol::Context
};

use super::Sockets;

pub struct Socket {
    family: ProtocolFamily,
    sock_type: SocketType,
    fd: Id,
    _is_active: bool, // These three will be needed when it comes to implementing listening and accepting
    _is_bound: bool,
    _is_listening: bool,
    is_blocking: RwLock<bool>,
    local_addr: RwLock<Option<SocketAddress>>,
    remote_addr: RwLock<Option<SocketAddress>>,
    session: Arc<RwLock<Option<SharedSession>>>,
    messages: Arc<RwLock<VecDeque<Message>>>,
    notify: Notify,
    protocols: ProtocolMap
}

impl Socket {
    
    pub(super) fn new(domain: ProtocolFamily, sock_type: SocketType, fd: Id, protocols: ProtocolMap) -> Socket {
        Self {
            family: domain,
            sock_type,
            fd,
            _is_active: true,
            _is_bound: false,
            _is_listening: false,
            is_blocking: RwLock::new(true),
            local_addr: RwLock::new(None),
            remote_addr: RwLock::new(None),
            messages: Default::default(),
            notify: Notify::new(),
            session: Default::default(),
            protocols
        }
    }

    pub fn set_blocking(self: Arc<Self>, is_blocking: bool) {
        *self.is_blocking.write().unwrap() = is_blocking;
    }

    pub fn connect(self: Arc<Self>, sock_addr: SocketAddress) -> Result<(), SocketError> {
        if self.session.read().unwrap().is_some() {
            return Err(SocketError::AcceptError(String::from("Socket is already connected")));
        }
        *self.remote_addr.write().unwrap() = Some(sock_addr);
        let mut participants = Control::new();
        if let Some(local_addr) = *self.local_addr.read().unwrap() {
            match local_addr.address {
                IpAddress::IPv4(addr) => {
                    Ipv4::set_local_address(addr, &mut participants);
                }
                IpAddress::IPv6() => {
                    todo!();
                }
            }
            match self.sock_type {
                SocketType::SocketDatagram => {
                    Udp::set_local_port(local_addr.port, &mut participants);
                }
                SocketType::SocketStream => {
                    todo!();
                }
            }
        }
        if let Some(remote_addr) = *self.remote_addr.read().unwrap() {
            match remote_addr.address {
                IpAddress::IPv4(addr) => {
                    Ipv4::set_remote_address(addr, &mut participants);
                }
                IpAddress::IPv6() => {
                    todo!();
                }
            }
            match self.sock_type {
                SocketType::SocketDatagram => {
                    Udp::set_remote_port(remote_addr.port, &mut participants);
                }
                SocketType::SocketStream => {
                    todo!();
                }
            }
        }
        let session = match self.protocols.protocol(Sockets::ID).expect("Sockets API not found").open(self.fd, participants, self.protocols.clone()) {
            Ok(v) => v,
            Err(e) => return Err(SocketError::ConnectError(e.to_string()))
        };
        *self.session.write().unwrap() = Some(session);
        Ok(())
    }

    pub fn bind(self: Arc<Self>, sock_addr: SocketAddress) -> Result<(), SocketError> {
        match self.family {
            ProtocolFamily::LOCAL => {
                return Err(SocketError::BindError(String::from("Cannot bind a local socket")));
            }
            ProtocolFamily::INET => {
                match sock_addr.address {
                    IpAddress::IPv4(_v) => {
                        *self.local_addr.write().unwrap() = Some(sock_addr)
                    }
                    IpAddress::IPv6() => {
                        return Err(SocketError::BindError(String::from("Cannot bind an INET socket to an IPv6 address")))
                    }
                }
            }
            ProtocolFamily::INET6 => {
                match sock_addr.address {
                    IpAddress::IPv4(_v) => {
                        return Err(SocketError::BindError(String::from("Cannot bind an INET6 socket to an IPv4 address")))
                    }
                    IpAddress::IPv6() => {
                        *self.local_addr.write().unwrap() = Some(sock_addr)
                    }
                }
            }
        }
        // self._is_bound = true;
        Ok(())
    }

    pub fn listen(&mut self, _backlog: i32) -> Result<(), SocketError> {
        todo!();
    }

    pub fn accept(&mut self) -> Result<Socket, SocketError> {
        todo!();
    }

    pub fn send(self: Arc<Self>, message: impl Into<Chunk> + std::marker::Send + 'static) -> Result<(), SocketError> {
        if self.session.read().unwrap().is_none() {
            return Err(SocketError::SendError(String::from("Socket isn't connected")));
        }
        let context = Context::new(self.protocols.clone());
        let session = self.session.clone();
        tokio::spawn(async move {
            session
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
            .send(Message::new(message), context)
            .unwrap();
        });
        Ok(())
    }

    pub async fn recv(self: Arc<Self>, bytes: usize) -> Result<Vec<u8>, SocketError> {
        if self.session.read().unwrap().is_none() {
            return Err(SocketError::ReceiveError(String::from("Socket isn't connected")));
        }
        if *self.is_blocking.read().unwrap() { self.notify.notified().await; }
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
        if !queue.is_empty() { self.notify.notify_one(); }
        Ok(buf)
    }

    pub async fn recv_msg(self: Arc<Self>) -> Result<Message, SocketError> {
        if self.session.read().unwrap().is_none() {
            return Err(SocketError::ReceiveError(String::from("Socket isn't connected")));
        }
        if *self.is_blocking.read().unwrap() { self.notify.notified().await; }
        let mut queue = self.messages.write().unwrap().clone();
        let msg = match queue.pop_front() {
            Some(v) => v,
            None => return Err(SocketError::Other(String::from("Message queue empty"))),
        };
        if !queue.is_empty() {self.notify.notify_one(); }
        Ok(msg)
    }

    pub(crate) fn receive(&self, message: Message) -> Result<(), SocketError> {
        self.messages.write().unwrap().push_back(message);
        self.notify.notify_one();
        Ok(())
    }
}

#[derive(Debug, ThisError, Clone, PartialEq, Eq)]
pub enum SocketError {
    #[error("Bind error")]
    BindError(String),
    #[error("Connect error")]
    ConnectError(String),
    #[error("Listen error")]
    ListenError(String),
    #[error("Accept error")]
    AcceptError(String),
    #[error("Send error")]
    SendError(String),
    #[error("Receive error")]
    ReceiveError(String),
    #[error("Unspecified error")]
    Other(String),
}

#[derive(Clone, Copy)]
pub enum ProtocolFamily {
    LOCAL,
    INET,
    INET6
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SocketType {
    SocketStream,
    SocketDatagram
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddress {
    IPv4(Ipv4Address),
    IPv6()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketAddress {
    address: IpAddress,
    port: u16
}

impl SocketAddress {
    pub fn new(address: IpAddress, port: u16) -> SocketAddress {
        match address {
            IpAddress::IPv4(addr) => SocketAddress::new_v4(addr, port),
            IpAddress::IPv6() => todo!(),
        }
    }

    pub fn new_v4(address: Ipv4Address, port: u16) -> SocketAddress {
        Self { address: IpAddress::IPv4(address), port }
    }

    pub fn new_v6(port: u16) -> SocketAddress {
        Self { address: IpAddress::IPv6(), port }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketId {
    local_address: SocketAddress,
    remote_address: SocketAddress
}

impl SocketId {
    pub fn new(local_address: IpAddress, local_port: u16, remote_address: IpAddress, remote_port: u16) -> SocketId {
        Self {
            local_address: SocketAddress::new(local_address, local_port),
            remote_address: SocketAddress::new(remote_address, remote_port)
        }
    }
}