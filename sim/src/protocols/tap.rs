use crate::core::{
    ArcSession, Control, ControlFlow, ControlKey, Message, Mtu, NetworkLayer, NetworkLayerError,
    PrimitiveError, Protocol, ProtocolContext, ProtocolContextError, ProtocolId, Session,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    mem,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

type NetworkIndex = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId {
    upstream: ProtocolId,
    network: NetworkIndex,
}

impl SessionId {
    pub fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self { upstream, network }
    }
}

/// Represents something akin to an Ethernet tap or a network interface card.
/// This should be the first responder to messages coming in off the network. It
/// is simply there to specify which protocol should respond to a raw message
/// coming off the network, for example IPv4 or IPv6. The header is very simple,
/// adding only a u32 that specifies the `ProtocolId` of the protocol that
/// should receive the message.
pub struct Tap {
    // Todo: Add an interface for accessing the MTUs
    // network_mtus: Vec<Mtu>,
    sessions: HashMap<SessionId, Arc<RwLock<TapSession>>>,
}

impl Tap {
    /// The unique identifier for this protocol
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    // Todo: We're going to want to use this parameter to initialize network_mtus on
    // the struct when we get around to it
    pub fn new(_network_mtus: Vec<Mtu>) -> Self {
        Self {
            sessions: Default::default(),
        }
    }

    pub fn outgoing(&mut self) -> Vec<(NetworkIndex, Vec<Message>)> {
        self.sessions
            .values()
            .map(|session| {
                let mut session = session.write().unwrap();
                (session.network(), session.outgoing())
            })
            .collect()
    }

    pub fn accept_incoming(
        &mut self,
        message: Message,
        network: NetworkIndex,
        mut context: ProtocolContext,
    ) -> Result<(), TapError> {
        let header = take_header(&message).ok_or(TapError::HeaderLength)?;
        let protocol_id: ProtocolId = header.try_into()?;
        let protocol = context.protocol(protocol_id)?;
        let mut protocol = protocol.write().unwrap();
        context
            .info()
            .insert(ControlKey::NetworkIndex, network);
        let message = message.slice(2..);
        let session_id = SessionId::new(protocol_id, network);
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let session = Arc::new(RwLock::new(TapSession::new(protocol_id, network)));
                entry.insert(session.clone());
                session
            }
        };
        protocol.demux(message, session, context)?;
        Ok(())
    }
}

impl Protocol for Tap {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let network = get_network_index(&participants)?;
        let session_id = SessionId::new(upstream, network);
        match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let session = Arc::new(RwLock::new(TapSession::new(upstream, network)));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // This is a no-op because nobody can call open_passive on us anyway
        Ok(())
    }

    fn demux(
        &mut self,
        _message: Message,
        _downstream: ArcSession,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no protocols under
        // this one that would ask Tap to demux a message and because, semantically,
        // demux chooses one of its own sessions to respond to the message. We want Tap
        // to immediatly forward incoming messages to a higher-up protocol.
        Err(Box::new(TapError::Demux))
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        for session in self.sessions.values_mut() {
            session
                .write()
                .unwrap()
                .awake(session.clone(), context.clone())?;
        }
        Ok(ControlFlow::Continue)
    }
}

fn get_network_index(control: &Control) -> Result<u8, TapError> {
    Ok(control
        .get(&ControlKey::NetworkIndex)
        .ok_or(TapError::IdentifierMissingKey(ControlKey::NetworkIndex))?
        .to_u8()?)
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}

#[derive(Clone)]
pub struct TapSession {
    network: NetworkIndex,
    outgoing: Vec<Message>,
    upstream: ProtocolId,
}

impl TapSession {
    fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self {
            upstream,
            network,
            outgoing: vec![],
        }
    }

    pub fn network(&self) -> NetworkIndex {
        self.network
    }

    pub fn outgoing(&mut self) -> Vec<Message> {
        mem::take(&mut self.outgoing)
    }
}

impl Session for TapSession {
    fn protocol(&self) -> ProtocolId {
        Tap::ID
    }

    fn send(
        &mut self,
        _self_handle: ArcSession,
        message: Message,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let header: [u8; 2] = self.upstream.into();
        let message = message.with_header(&header);
        self.outgoing.push(message);
        Ok(())
    }

    fn recv(
        &mut self,
        _self_handle: ArcSession,
        _message: Message,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(Box::new(TapError::Recv))
    }

    fn awake(
        &mut self,
        _self_handle: ArcSession,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("The header did not represent a valid protocol ID")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Unexpected passive open")]
    PassiveOpen,
    #[error("Attempt to create an existing demux binding: {0:?}")]
    BindingExists(ProtocolId),
    #[error("Could not find a matching session")]
    SessionNotFound,
    #[error("An identifier is missing a required key")]
    IdentifierMissingKey(ControlKey),
    #[error("The network index does not exist: {0}")]
    NetworkIndex(NetworkIndex),
    #[error("New messages should go directly to the protocol, not the session")]
    Recv,
    #[error("Cannot demux because the incoming method should be used instead")]
    Demux,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
    #[error("{0}")]
    ProtocolContext(#[from] ProtocolContextError),
}
