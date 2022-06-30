use crate::core::{
    ArcSession, Control, ControlFlow, ControlKey, Message, NetworkLayer, PrimitiveError, Protocol,
    ProtocolContext, ProtocolId, Session,
};
use etherparse::{IpNumber, Ipv4Header, Ipv4HeaderSlice};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;
use super::Nic;

pub struct Ipv4 {
    listen_bindings: HashMap<u32, ProtocolId>,
    sessions: HashMap<Identifier, ArcSession>,
}

impl Ipv4 {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Network, 4);
}

impl Protocol for Ipv4 {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        mut participants: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let key = Identifier::from_identifier(&participants)?;
        match self.sessions.entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.source, key.destination))?,
            Entry::Vacant(entry) => {
                // Todo: Actually pick the right network index
                participants.insert(ControlKey::NetworkIndex, 0.into());
                let nic_session = context
                    .protocol(Nic::ID)?
                    .write()
                    .unwrap()
                    .open_active(Self::ID, participants, context)?;
                let session = Arc::new(RwLock::new(Ipv4Session::new(nic_session, upstream, key)));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn open_passive(
        &mut self,
        downstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        todo!()
    }

    fn listen(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let source = get_source(&participants)?;
        match self.listen_bindings.entry(source) {
            Entry::Occupied(_) => Err(Ipv4Error::BindingExists(source))?,
            Entry::Vacant(entry) => {
                entry.insert(upstream);
            }
        }
        Ok(())
    }

    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }

    fn get_session(&self, identifier: &Control) -> Result<ArcSession, Box<dyn Error>> {
        let key = Identifier::from_identifier(identifier)?;
        Ok(self
            .sessions
            .get(&key)
            .ok_or(Ipv4Error::MissingSession(key.source, key.destination))?
            .clone())
    }
}

pub struct Ipv4Session {
    upstream: ProtocolId,
    downstream: ArcSession,
    identifier: Identifier,
}

impl Ipv4Session {
    fn new(downstream: ArcSession, upstream: ProtocolId, identifier: Identifier) -> Self {
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

    fn send(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
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
            _ => Err(Ipv4Error::UnknownUpstreamProtocol)?,
        };

        let mut header = Ipv4Header::new(
            length as u16,
            30,
            ip_number,
            self.identifier.source.to_be_bytes(),
            self.identifier.destination.to_be_bytes(),
        );
        header.header_checksum = header.calc_header_checksum()?;

        let mut header_buffer = vec![];
        header.write(&mut header_buffer)?;

        let message = message.with_header(header_buffer);
        self.downstream.write().unwrap().send(message, context)?;
        Ok(())
    }

    fn recv(
        &mut self,
        message: Message,
        mut context: ProtocolContext,
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
            ControlKey::SourceAddress,
            u32::from_be_bytes(header.source()).into(),
        );
        info.insert(
            ControlKey::DestinationAddress,
            u32::from_be_bytes(header.destination()).into(),
        );
        let message = message.slice(20..);
        context
            .protocol(self.upstream)?
            .read()
            .unwrap()
            .demux(message, context);
        Ok(())
    }

    fn awake(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum Ipv4Error {
    #[error("The identifier for a demux binding was missing a source address")]
    MissingSourceAddress,
    #[error("The identifier for a demux binding was missing a destination address")]
    MissingDestinationAddress,
    #[error("Attempting to create a binding that already exists for source address {0:#010x}")]
    BindingExists(u32),
    #[error("Attempting to create a session that already exists for {0:#010x} -> {1:#010x}")]
    SessionExists(u32, u32),
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
    #[error("Could not find a session for the key {0:#010x} -> {1:010x}")]
    MissingSession(u32, u32),
    #[error("Did not recognize the upstream protocol")]
    UnknownUpstreamProtocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Identifier {
    pub source: u32,
    pub destination: u32,
}

impl Identifier {
    pub fn new(source: u32, destination: u32) -> Self {
        Self {
            source,
            destination,
        }
    }

    pub fn from_identifier(control: &Control) -> Result<Self, Ipv4Error> {
        Ok(Self::new(get_source(control)?, get_destination(control)?))
    }
}

fn get_source(control: &Control) -> Result<u32, Ipv4Error> {
    Ok(control
        .get(&ControlKey::SourceAddress)
        .ok_or(Ipv4Error::MissingSourceAddress)?
        .to_u32()?)
}

fn get_destination(control: &Control) -> Result<u32, Ipv4Error> {
    Ok(control
        .get(&&ControlKey::DestinationAddress)
        .ok_or(Ipv4Error::MissingDestinationAddress)?
        .to_u32()?)
}
