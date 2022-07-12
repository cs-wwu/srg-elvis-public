use crate::core::{
    message::Message, Control, ControlFlow, Mtu, Protocol, ProtocolContext, ProtocolId,
    SharedSession,
};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod tap_misc;
pub use tap_misc::NetworkIndex;

mod tap_session;
pub use tap_session::TapSession;

use self::{tap_misc::TapError, tap_session::SessionId};

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
    pub const ID: ProtocolId = ProtocolId::of::<Self>();

    // Todo: We're going to want to use this parameter to initialize
    // network_mtus on the struct when we get around to it
    pub fn new(network_mtus: Vec<Mtu>) -> Self {
        Self {
            network_mtus,
            sessions: Default::default(),
        }
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
        network: u8,
        context: &mut ProtocolContext,
    ) -> Result<(), TapError> {
        let header = take_header(&message).ok_or(TapError::HeaderLength)?;
        NetworkIndex::set(&mut context.info, network);
        let message = message.slice(8..);
        let session_id = SessionId::new(header, network.into());
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let session = Rc::new(RefCell::new(TapSession::new(header, network.into())));
                entry.insert(session.clone());
                session
            }
        };
        let mut session = SharedSession::from(session);
        session.receive(message, context)?;
        Ok(())
    }
}

impl Protocol for Tap {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let network = NetworkIndex::get(&participants);
        let session_id = SessionId::new(upstream, network.into());
        match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone().into()),
            Entry::Vacant(entry) => {
                let session = Rc::new(RefCell::new(TapSession::new(upstream, network.into())));
                entry.insert(session.clone());
                Ok(session.into())
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
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no
        // protocols under this one that would ask Tap to demux a message and
        // because, semantically, demux chooses one of its own sessions to
        // respond to the message. We want Tap to immediatly forward incoming
        // messages to a higher-up protocol.
        panic!("Cannot demux on a Tap")
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

fn take_header(message: &Message) -> Option<ProtocolId> {
    let mut iter = message.iter();
    Some(
        u64::from_be_bytes([
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
        ])
        .into(),
    )
}
