use crate::core::{
    message::Message, Control, ControlFlow, Mtu, NetworkLayer, Protocol, ProtocolContext,
    ProtocolId, RcSession,
};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod tap_misc;
pub use tap_misc::NETWORK_INDEX_KEY;

mod tap_session;
pub use tap_session::TapSession;

use self::{
    tap_misc::{NetworkIndex, TapError},
    tap_session::SessionId,
};

/// Represents something akin to an Ethernet tap or a network interface card.
/// This should be the first responder to messages coming in off the network. It
/// is simply there to specify which protocol should respond to a raw message
/// coming off the network, for example IPv4 or IPv6. The header is very simple,
/// adding only a u32 that specifies the `ProtocolId` of the protocol that
/// should receive the message.
pub struct Tap {
    // Todo: Add an interface for accessing the MTUs
    #[allow(dead_code)]
    network_mtus: Vec<Mtu>,
    sessions: HashMap<SessionId, Rc<RefCell<TapSession>>>,
}

impl Tap {
    /// The unique identifier for this protocol
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    // Todo: We're going to want to use this parameter to initialize network_mtus on
    // the struct when we get around to it
    pub fn new(network_mtus: Vec<Mtu>) -> Self {
        Self {
            network_mtus,
            sessions: Default::default(),
        }
    }

    pub fn new_shared(network_mtus: Vec<Mtu>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(network_mtus)))
    }

    pub fn outgoing(&mut self) -> Vec<(NetworkIndex, Vec<Message>)> {
        self.sessions
            .values()
            .map(|session| {
                let mut session = session.borrow_mut();
                (session.network(), session.outgoing())
            })
            .collect()
    }

    pub fn accept_incoming(
        &mut self,
        message: Message,
        network: NetworkIndex,
        context: &mut ProtocolContext,
    ) -> Result<(), TapError> {
        let header = take_header(&message).ok_or(TapError::HeaderLength)?;
        let protocol_id: ProtocolId = header.try_into()?;
        let protocol = context.protocol(protocol_id)?;
        context.info().insert(NETWORK_INDEX_KEY, network);
        let message = message.slice(2..);
        let session_id = SessionId::new(protocol_id, network);
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let session = Rc::new(RefCell::new(TapSession::new(protocol_id, network)));
                entry.insert(session.clone());
                session
            }
        };
        protocol.borrow_mut().demux(message, session, context)?;
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
        _context: &mut ProtocolContext,
    ) -> Result<RcSession, Box<dyn Error>> {
        let network = participants
            .get(NETWORK_INDEX_KEY)
            .expect("Missing network index")
            .to_u8()
            .expect("Incorrect network index type");
        let session_id = SessionId::new(upstream, network);
        match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let session = Rc::new(RefCell::new(TapSession::new(upstream, network)));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // This is a no-op because nobody can call open_passive on us anyway
        Ok(())
    }

    fn demux(
        &mut self,
        _message: Message,
        _downstream: RcSession,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no protocols under
        // this one that would ask Tap to demux a message and because, semantically,
        // demux chooses one of its own sessions to respond to the message. We want Tap
        // to immediatly forward incoming messages to a higher-up protocol.
        panic!("Cannot demux on a Tap")
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}
