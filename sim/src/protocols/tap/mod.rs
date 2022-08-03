//! The base-level protocol that communicates directly with networks.

use crate::core::{
    message::Message, Control, ControlFlow, Mtu, Network, Protocol, ProtocolContext, ProtocolId,
    SharedSession,
};
use std::{
    cell::{Ref, RefCell},
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod tap_misc;
pub use tap_misc::NetworkIndex;

mod tap_session;
use tap_session::TapSession;

use self::{tap_misc::TapError, tap_session::SessionId};

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
pub struct Tap {
    // TODO(hardint): Add an interface for accessing the MTUs
    #[allow(dead_code)]
    network_mtus: Vec<Mtu>,
    sessions: HashMap<SessionId, Rc<RefCell<TapSession>>>,
}

impl Tap {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(0xdeadbeef);

    /// Creates a new network tap.
    pub fn new() -> Self {
        Self {
            network_mtus: vec![],
            sessions: Default::default(),
        }
    }

    pub fn attach(&mut self, network: Ref<Network>) {
        // TODO(hardint): Also store a channel to send on
        self.network_mtus.push(network.mtu());
    }

    /// Gets a list of the pending, outgoing messages that have been sent on the
    /// tap.
    pub fn outgoing(&mut self) -> Vec<(NetworkIndex, Vec<Message>)> {
        self.sessions
            .values()
            .map(|session| {
                let mut session = session.borrow_mut();
                (session.network(), session.outgoing())
            })
            .collect()
    }

    /// Delivers a message to the network for delivery up the protocol stack.
    /// The tap will demux the message and forward it to the appropriate
    /// protocol.
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
