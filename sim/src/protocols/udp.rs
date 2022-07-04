use super::{Ipv4, Ipv4Address};
use crate::core::{
    Control, ControlFlow, ControlKey, Message, NetworkLayer, PrimitiveError, Protocol,
    ProtocolContext, ProtocolId, RcSession, Session,
};
use core::slice::SlicePattern;
use etherparse::{Ipv4Header, UdpHeader, UdpHeaderSlice};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};
use thiserror::Error as ThisError;

#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: HashMap<ListenId, ProtocolId>,
    sessions: HashMap<SessionId, Rc<RefCell<UdpSession>>>,
}

impl Udp {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Transport, 17);

    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }
}

impl Protocol for Udp {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<RcSession, Box<dyn Error>> {
        let local_port = get_local_port(&participants)?;
        let remote_port = get_remote_port(&participants)?;
        let local_address = get_local_address(&participants)?;
        let remote_address = get_remote_address(&participants)?;
        let identifier = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        match self.sessions.entry(identifier) {
            Entry::Occupied(_) => Err(UdpError::SessionExists)?,
            Entry::Vacant(entry) => {
                let downstream = context.protocol(Ipv4::ID)?.borrow_mut().open_active(
                    Self::ID,
                    participants,
                    context,
                )?;
                let session = UdpSession::new_shared(upstream, downstream, identifier);
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let port = get_local_port(&participants)?;
        let address = get_local_address(&participants)?;
        let identifier = ListenId { address, port };
        self.listen_bindings.insert(identifier, upstream);
        Ok(())
    }

    fn demux(
        &mut self,
        message: Message,
        downstream: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // Todo: Scuffed copy fest. Revise.
        let header_bytes: Vec<_> = message.iter().take(64).collect();
        let header = UdpHeaderSlice::from_slice(header_bytes.as_slice())?;
        let local_address = get_local_address(context.info())?;
        let remote_address = get_remote_address(context.info())?;
        let local_port = header.destination_port();
        let remote_port = header.source_port();
        let session_id = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        context.info().insert(ControlKey::LocalPort, local_port);
        context.info().insert(ControlKey::RemotePort, remote_port);
        let message = message.slice(64..);
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                let listen_id = ListenId {
                    address: local_address,
                    port: local_port,
                };
                match self.listen_bindings.entry(listen_id) {
                    Entry::Occupied(listen_entry) => {
                        let session =
                            UdpSession::new_shared(*listen_entry.get(), downstream, session_id);
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => Err(UdpError::MissingSession)?,
                }
            }
        };
        session.borrow_mut().send(session.clone(), message, context);
        Ok(())
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

fn get_local_port(control: &Control) -> Result<u16, UdpError> {
    Ok(control
        .get(&ControlKey::LocalPort)
        .ok_or(UdpError::MissingLocalPort)?
        .to_u16()?)
}

fn get_remote_port(control: &Control) -> Result<u16, UdpError> {
    Ok(control
        .get(&ControlKey::RemotePort)
        .ok_or(UdpError::MissingRemotePort)?
        .to_u16()?)
}

fn get_local_address(control: &Control) -> Result<Ipv4Address, UdpError> {
    Ok(control
        .get(&ControlKey::LocalAddress)
        .ok_or(UdpError::MissingLocalAddress)?
        .to_u32()?
        .into())
}

fn get_remote_address(control: &Control) -> Result<Ipv4Address, UdpError> {
    Ok(control
        .get(&ControlKey::RemoteAddress)
        .ok_or(UdpError::MissingRemoteAddress)?
        .to_u32()?
        .into())
}

pub struct UdpSession {
    upstream: ProtocolId,
    downstream: RcSession,
    identifier: SessionId,
}

impl UdpSession {
    fn new(upstream: ProtocolId, downstream: RcSession, identifier: SessionId) -> Self {
        Self {
            upstream,
            downstream,
            identifier,
        }
    }

    fn new_shared(
        upstream: ProtocolId,
        downstream: RcSession,
        identifier: SessionId,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(upstream, downstream, identifier)))
    }
}

impl Session for UdpSession {
    fn protocol(&self) -> ProtocolId {
        Udp::ID
    }

    fn send(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let id = self.identifier;
        let payload_len = message.iter().count();
        // Todo: We want to use the checksum
        let header = UdpHeader::without_ipv4_checksum(id.local_port, id.remote_port, payload_len)?;
        let mut header_bytes = vec![];
        header.write(&mut header_bytes);
        let message = message.with_header(header_bytes);
        self.downstream
            .borrow_mut()
            .send(self.downstream.clone(), message, context);
        Ok(())
    }

    fn recv(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context
            .protocol(self.upstream)?
            .borrow_mut()
            .demux(message, self_handle, context)?;
        Ok(())
    }

    fn awake(
        &mut self,
        self_handle: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId {
    local_address: Ipv4Address,
    local_port: u16,
    remote_address: Ipv4Address,
    remote_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    address: Ipv4Address,
    port: u16,
}

#[derive(Debug, ThisError)]
pub enum UdpError {
    #[error("Could not get the local port number")]
    MissingLocalPort,
    #[error("Could not get the remote port number")]
    MissingRemotePort,
    #[error("Could not get the local address number")]
    MissingLocalAddress,
    #[error("Could not get the remote address number")]
    MissingRemoteAddress,
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
}
