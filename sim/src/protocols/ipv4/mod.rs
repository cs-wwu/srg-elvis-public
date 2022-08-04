//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    core::{message::Message, Control, Protocol, ProtocolContext, ProtocolId, SharedSession},
    protocols::tap::Tap,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    sync::{Arc, Mutex},
};

mod ipv4_parsing;
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_misc;
use ipv4_misc::Ipv4Error;
pub use ipv4_misc::{LocalAddress, RemoteAddress};

mod ipv4_session;
use ipv4_session::{Ipv4Session, SessionId};

use super::tap::NetworkId;

/// An implementation of the Internet Protocol.
#[derive(Default, Clone)]
pub struct Ipv4 {
    listen_bindings: HashMap<LocalAddress, ProtocolId>,
    sessions: HashMap<SessionId, SharedSession>,
}

impl Ipv4 {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(4);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
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
        let local = LocalAddress::try_from(&participants).unwrap();
        let remote = RemoteAddress::try_from(&participants).unwrap();
        let key = SessionId { local, remote };
        match self.sessions.entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.local, key.remote))?,
            Entry::Vacant(entry) => {
                // TODO(hardint): Actually pick the right network index
                NetworkId::set(&mut participants, 0);
                let tap_session = context
                    .protocol(Tap::ID)
                    .expect("No such protocol")
                    .lock()
                    .unwrap()
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
        let local = LocalAddress::try_from(&participants).unwrap();
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
            .lock()
            .unwrap()
            .listen(Self::ID, participants, context)
    }

    fn demux(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let header = Ipv4Header::from_bytes(message.iter())?;
        let remote = RemoteAddress::from(header.source);
        let local = LocalAddress::from(header.destination);
        let identifier = SessionId { local, remote };
        local.apply(&mut context.info);
        remote.apply(&mut context.info);
        let message = message.slice(header.ihl as usize * 4..);
        let mut session = match self.sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => match self.listen_bindings.get(&local) {
                Some(&binding) => {
                    let session = SharedSession::new(Ipv4Session::new(
                        context.current_session().expect("No current session"),
                        binding,
                        identifier,
                    ));
                    entry.insert(session.clone());
                    session
                }
                None => Err(Ipv4Error::MissingListenBinding(local))?,
            },
        };
        session.receive(message, context)?;
        Ok(())
    }

    fn start(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
