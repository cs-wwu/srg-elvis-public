use crate::{
    core::{
        message::Message, Control, ControlFlow, NetworkLayer, Protocol, ProtocolContext,
        ProtocolId, SharedSession,
    },
    protocols::tap::Tap,
};
use etherparse::Ipv4HeaderSlice;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_misc;
use ipv4_misc::Ipv4Error;
pub use ipv4_misc::{LocalAddress, RemoteAddress};

mod ipv4_session;
pub use ipv4_session::Ipv4Session;
use ipv4_session::SessionId;

use super::tap::set_network_index;

#[derive(Default, Clone)]
pub struct Ipv4 {
    listen_bindings: HashMap<Ipv4Address, ProtocolId>,
    sessions: HashMap<SessionId, SharedSession>,
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

    fn open(
        &mut self,
        upstream: ProtocolId,
        mut participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let local = LocalAddress::get(&participants);
        let remote = RemoteAddress::get(&participants);
        let key = SessionId::new(local, remote);
        match self.sessions.entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.local, key.remote))?,
            Entry::Vacant(entry) => {
                // Todo: Actually pick the right network index
                set_network_index(&mut participants, 0u8);
                let tap_session = context
                    .protocol(Tap::ID)
                    .expect("No such protocol")
                    .borrow_mut()
                    .open(Self::ID, participants, context)?;
                let session = SharedSession::new(Ipv4Session::new(tap_session, upstream, key));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let local = LocalAddress::get(&participants);
        match self.listen_bindings.entry(local) {
            Entry::Occupied(_) => Err(Ipv4Error::BindingExists(local))?,
            Entry::Vacant(entry) => {
                entry.insert(upstream);
            }
        }

        // Essentially a no-op but good for completeness and as an example
        context
            .protocol(Tap::ID)
            .expect("No such protocol")
            .borrow_mut()
            .listen(Self::ID, participants, context)
    }

    fn demux(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // Todo: This is going to kind of scuffed for the time being. Etherparse
        // makes my work a lot easier but it also demands a slice to operate on,
        // which the Message API doesn't offer. We're going to break zero-copy a
        // bit and just copy the first twenty bytes of the message to treat as
        // the header. In the future, we're going to want to replace Etherparse
        // with our own parsing code so we can just work with the iterator API
        // directly.
        let header: Vec<_> = message.iter().take(20).collect();
        let header = Ipv4HeaderSlice::from_slice(&header)?;
        let source = Ipv4Address::new(header.source());
        let destination = Ipv4Address::new(header.destination());
        let identifier = SessionId::new(destination, source);
        LocalAddress::set(&mut context.info, destination);
        RemoteAddress::set(&mut context.info, source);
        let message = message.slice(20..);
        let mut session = match self.sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                match self.listen_bindings.get(&destination) {
                    Some(&binding) => {
                        // Todo: We want to be zero-copy, but right now it
                        // requires copying to forward the list of participants.
                        // Is there any way around this?
                        let session = SharedSession::new(Ipv4Session::new(
                            context.current_session().expect("No current session"),
                            binding,
                            identifier,
                        ));
                        entry.insert(session.clone());
                        session
                    }
                    None => Err(Ipv4Error::MissingListenBinding(destination))?,
                }
            }
        };
        session.recv(message, context)?;
        Ok(())
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}
