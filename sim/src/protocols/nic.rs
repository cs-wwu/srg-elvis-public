use crate::core::{
    ArcProtocol, ArcSession, Control, ControlFlow, ControlKey, Message, Mtu, NetworkLayer,
    NetworkLayerError, Primitive, Protocol, ProtocolContext, ProtocolId, Session,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    mem,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

/// Represents something akin to an Ethernet tap or a network interface card.
/// This should be the first responder to messages coming in off the network. It
/// is simply there to specify which protocol should respond to a raw message
/// coming off the network, for example IPv4 or IPv6. The header is very simple,
/// adding only a u32 that specifies the `ProtocolId` of the protocol that
/// should receive the message.
pub struct Nic {
    network_mtus: Vec<Mtu>,
    bindings: HashMap<ProtocolId, ArcProtocol>,
    sessions: HashMap<ProtocolId, Arc<RwLock<NicSession>>>,
}

impl Nic {
    /// The unique identifier for this protocol
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    /// Creates a new network interface card.
    ///
    /// # Arguments
    ///
    /// * `mtu` is the minimum transmission unit of the connected network. It is
    ///   the number of bytes in the largest frame the network supports.
    /// * `network_index` is the index of the network this NIC attaches to. When
    ///   using the neighbors iterator on
    ///   [AwakeContext](elvis::core::AwakeContext), the network index refers to
    ///   the nth element of the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use elvis::protocols::Nic;
    /// let _nic = Nic::new(1500, 0);
    /// ```
    pub fn new(network_mtus: Vec<Mtu>) -> Self {
        Self {
            network_mtus,
            bindings: Default::default(),
            sessions: Default::default(),
        }
    }

    pub fn messages(&mut self) -> Vec<(u8, Vec<Message>)> {
        self.sessions
            .values()
            .map(|session| {
                let mut session = session.write().unwrap();
                (session.network_index(), session.outgoing())
            })
            .collect()
    }
}

impl Protocol for Nic {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        requester: ArcSession,
        identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let network_index = match identifier
            .get(&ControlKey::NetworkIndex)
            .ok_or(NicError::IdentifierMissingNetworkIndex)?
        {
            Primitive::U8(index) => *index,
            _ => Err(NicError::IdentifierMissingNetworkIndex)?,
        };
        let protocol = requester.read().unwrap().protocol();
        match self.sessions.entry(protocol) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let session = Arc::new(RwLock::new(NicSession::new(requester, network_index)));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn open_passive(
        &mut self,
        _invoker: ArcSession,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(Box::new(NicError::PassiveOpen))
    }

    fn add_demux_binding(
        &mut self,
        requester: ArcProtocol,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let id = requester.read().unwrap().id();
        match self.bindings.entry(id) {
            Entry::Occupied(_) => Err(Box::new(NicError::BindingExists(id))),
            Entry::Vacant(entry) => {
                entry.insert(requester.clone());
                Ok(())
            }
        }
    }

    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        let header = take_header(&message).ok_or(NicError::HeaderLength)?;
        let protocol: ProtocolId = header.try_into()?;
        let session = self
            .sessions
            .get(&protocol)
            .ok_or(NicError::NoSessionForHeader)?;
        session.write().unwrap().recv(message, context)?;
        Ok(())
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        for session in self.sessions.values_mut() {
            session.write().unwrap().awake(context.clone())?;
        }
        Ok(ControlFlow::Continue)
    }
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}

#[derive(Clone)]
pub struct NicSession {
    network_index: u8,
    outgoing: Vec<Message>,
    upstream: ArcSession,
}

impl NicSession {
    fn new(upstream: ArcSession, network_index: u8) -> Self {
        Self {
            network_index,
            upstream,
            outgoing: vec![],
        }
    }

    pub fn network_index(&self) -> u8 {
        self.network_index
    }

    pub fn outgoing(&mut self) -> Vec<Message> {
        mem::take(&mut self.outgoing)
    }
}

impl Session for NicSession {
    fn protocol(&self) -> ProtocolId {
        Nic::ID
    }

    fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        let header: [u8; 2] = self.upstream.read().unwrap().protocol().into();
        let message = message.with_header(&header);
        self.outgoing.push(message);
        Ok(())
    }

    fn recv(&mut self, _message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Err(Box::new(NicError::Recv))
    }

    fn awake(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum NicError {
    #[error("Expected two bytes for the NIC header")]
    HeaderLength,
    #[error("The NIC header did not represent a valid protocol ID")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Unexpected passive open on NIC")]
    PassiveOpen,
    #[error("Attempt to create an existing demux binding: {0:?}")]
    BindingExists(ProtocolId),
    #[error("Could not find a matching session for the NIC header")]
    NoSessionForHeader,
    #[error("Missing a network index for creating session")]
    IdentifierMissingNetworkIndex,
    #[error("The network index does not exist: {0}")]
    NetworkIndex(u8),
    #[error("New messages on a NIC should go directly to the protocol, not the session")]
    Recv,
    #[error("{0}")]
    Other(Box<dyn Error>),
}
