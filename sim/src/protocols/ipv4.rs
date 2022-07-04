use super::Tap;
use crate::core::{
    Control, ControlFlow, ControlKey, Message, NetworkLayer, PrimitiveError, Protocol,
    ProtocolContext, ProtocolId, RcSession, Session,
};
use etherparse::{IpNumber, Ipv4Header, Ipv4HeaderSlice};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::Display,
    fmt::{self},
    rc::Rc,
};
use thiserror::Error as ThisError;

#[derive(Default, Clone)]
pub struct Ipv4 {
    listen_bindings: HashMap<Ipv4Address, ProtocolId>,
    sessions: HashMap<SessionId, RcSession>,
}

impl Ipv4 {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Network, 4);

    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }
}

impl Protocol for Ipv4 {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        mut participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<RcSession, Box<dyn Error>> {
        let local = get_local(&participants)?;
        let remote = get_remote(&participants)?;
        let key = SessionId::new(local, remote);
        match self.sessions.entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.local, key.remote))?,
            Entry::Vacant(entry) => {
                // Todo: Actually pick the right network index
                participants.insert(ControlKey::NetworkIndex, 0u8);
                let tap_session = context.protocol(Tap::ID)?.borrow_mut().open_active(
                    Self::ID,
                    participants,
                    context,
                )?;
                let session = Rc::new(RefCell::new(Ipv4Session::new(tap_session, upstream, key)));
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
        let local = get_local(&participants)?;
        match self.listen_bindings.entry(local) {
            Entry::Occupied(_) => Err(Ipv4Error::BindingExists(local))?,
            Entry::Vacant(entry) => {
                entry.insert(upstream);
            }
        }
        Ok(())
    }

    fn demux(
        &mut self,
        message: Message,
        downstream: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let header: Vec<_> = message.iter().take(20).collect();
        let header = Ipv4HeaderSlice::from_slice(&header)?;
        let source = Ipv4Address::new(header.source());
        let destination = Ipv4Address::new(header.destination());
        let identifier = SessionId::new(destination, source);
        let info = &mut context.info();
        info.insert(ControlKey::LocalAddress, destination.to_u32());
        info.insert(ControlKey::RemoteAddress, source.to_u32());
        match self.sessions.entry(identifier) {
            Entry::Occupied(entry) => {
                let session = entry.get();
                session
                    .borrow_mut()
                    .recv(session.clone(), message, context)?;
            }
            Entry::Vacant(entry) => {
                match self.listen_bindings.get(&destination) {
                    Some(&binding) => {
                        // Todo: We want to be zero-copy, but right now it requires copying to
                        // forward the list of participants. Is there any way around this?
                        let session = Rc::new(RefCell::new(Ipv4Session::new(
                            downstream.clone(),
                            binding,
                            identifier,
                        )));
                        session.borrow_mut().recv(downstream, message, context)?;
                        entry.insert(session);
                    }
                    None => Err(Ipv4Error::MissingListenBinding(destination))?,
                }
            }
        }
        Ok(())
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

pub struct Ipv4Session {
    upstream: ProtocolId,
    downstream: RcSession,
    identifier: SessionId,
}

impl Ipv4Session {
    fn new(downstream: RcSession, upstream: ProtocolId, identifier: SessionId) -> Self {
        Self {
            upstream,
            downstream,
            identifier,
        }
    }
}

impl Session for Ipv4Session {
    fn protocol(&self) -> ProtocolId {
        Ipv4::ID
    }

    fn send(
        &mut self,
        _self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let length = message.iter().count();
        let ip_number = match self.upstream {
            ProtocolId {
                layer: NetworkLayer::Transport,
                identifier: 6,
            } => IpNumber::Tcp,
            ProtocolId {
                layer: NetworkLayer::Transport,
                identifier: 17,
            } => IpNumber::Udp,
            // Todo: Ipv4 expects UDP or TCP upstream, so we gotta make that now
            _ => Err(Ipv4Error::UnknownUpstreamProtocol)?,
        };

        let mut header = Ipv4Header::new(
            length as u16,
            30,
            ip_number,
            self.identifier.local.into(),
            self.identifier.remote.into(),
        );
        header.header_checksum = header.calc_header_checksum()?;

        let mut header_buffer = vec![];
        header.write(&mut header_buffer)?;

        let message = message.with_header(header_buffer);
        self.downstream
            .borrow_mut()
            .send(self.downstream.clone(), message, context)?;
        Ok(())
    }

    fn recv(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // Todo: This is going to kind of scuffed for the time being. Etherparse makes
        // my work a lot easier but it also demands a slice to operate on, which the
        // Message API doesn't offer. We're going to break zero-copy a bit and just copy
        // the first twenty bytes of the message to treat as the header. In the future,
        // we're going to want to replace Etherparse with our own parsing code so we can
        // just work with the iterator API directly.
        let header: Vec<_> = message.iter().take(20).collect();
        let header = Ipv4HeaderSlice::from_slice(&header)?;
        let info = context.info();
        // Todo: Offer a better API for the Control type so we don't have to call
        // .into() on every primitive.
        info.insert(
            ControlKey::RemoteAddress,
            u32::from_be_bytes(header.source()),
        );
        info.insert(
            ControlKey::LocalAddress,
            u32::from_be_bytes(header.destination()),
        );
        let message = message.slice(20..);
        context
            .protocol(self.upstream)?
            .borrow_mut()
            .demux(message, self_handle, context)?;
        Ok(())
    }

    fn awake(
        &mut self,
        _self_handle: RcSession,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum Ipv4Error {
    #[error("Could not find a listen binding for the local address: {0}")]
    MissingListenBinding(Ipv4Address),
    #[error("The identifier for a demux binding was missing a source address")]
    MissingLocalAddress,
    #[error("The identifier for a demux binding was missing a destination address")]
    MissingRemoteAddress,
    #[error("Attempting to create a binding that already exists for source address {0}")]
    BindingExists(Ipv4Address),
    #[error("Attempting to create a session that already exists for {0} -> {1}")]
    SessionExists(Ipv4Address, Ipv4Address),
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
    #[error("Could not find a session for the key {0} -> {1}")]
    MissingSession(Ipv4Address, Ipv4Address),
    #[error("Did not recognize the upstream protocol")]
    UnknownUpstreamProtocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId {
    pub local: Ipv4Address,
    pub remote: Ipv4Address,
}

impl SessionId {
    pub fn new(local: Ipv4Address, remote: Ipv4Address) -> Self {
        Self { local, remote }
    }
}

// Todo: Semantics of source and destination per-callsite
fn get_local(control: &Control) -> Result<Ipv4Address, Ipv4Error> {
    Ok(control
        .get(&ControlKey::LocalAddress)
        .ok_or(Ipv4Error::MissingLocalAddress)?
        .to_u32()?
        .into())
}

fn get_remote(control: &Control) -> Result<Ipv4Address, Ipv4Error> {
    Ok(control
        .get(&ControlKey::RemoteAddress)
        .ok_or(Ipv4Error::MissingRemoteAddress)?
        .to_u32()?
        .into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    pub const CURRENT_NETWORK: Self = Self([0u8, 0, 0, 0]);
    pub const PRIVATE_NETWORK: Self = Self([10u8, 0, 0, 0]);
    pub const LOCALHOST: Self = Self([127u8, 0, 0, 1]);
    pub const SUBNET: Self = Self([255u8, 255, 255, 255]);

    pub fn new(address: impl Into<Self>) -> Self {
        address.into()
    }

    pub fn to_u32(self) -> u32 {
        self.into()
    }

    pub fn to_bytes(self) -> [u8; 4] {
        self.into()
    }
}

impl Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = <[u8; 4]>::from(*self);
        write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

impl From<u32> for Ipv4Address {
    fn from(n: u32) -> Self {
        Self::from(n.to_be_bytes())
    }
}

impl From<[u8; 4]> for Ipv4Address {
    fn from(n: [u8; 4]) -> Self {
        Self(n)
    }
}

impl From<Ipv4Address> for u32 {
    fn from(address: Ipv4Address) -> Self {
        u32::from_be_bytes(address.0)
    }
}

impl From<Ipv4Address> for [u8; 4] {
    fn from(address: Ipv4Address) -> Self {
        address.0
    }
}
