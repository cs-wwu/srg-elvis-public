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
use tokio::sync::mpsc::Sender;

use super::tap::NetworkId;

type ArcMap<K, V> = Arc<Mutex<HashMap<K, V>>>;
pub type IpToNetwork = HashMap<Ipv4Address, crate::core::NetworkId>;

/// An implementation of the Internet Protocol.
#[derive(Clone)]
pub struct Ipv4 {
    listen_bindings: ArcMap<LocalAddress, ProtocolId>,
    sessions: ArcMap<SessionId, SharedSession>,
    network_for_ip: Arc<Mutex<IpToNetwork>>,
}

impl Ipv4 {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(4);

    /// Creates a new instance of the protocol.
    pub fn new(network_for_ip: IpToNetwork) -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
            network_for_ip: Arc::new(Mutex::new(network_for_ip)),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared(network_for_ip: IpToNetwork) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(network_for_ip)))
    }
}

// TODO(hardint): Add a static IP lookup table in the constructor so that
// messages can be sent to the correct network

impl Protocol for Ipv4 {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open(
        &mut self,
        upstream: ProtocolId,
        mut participants: Control,
        context: ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let local = LocalAddress::try_from(&participants).unwrap();
        let remote = RemoteAddress::try_from(&participants).unwrap();
        let key = SessionId { local, remote };
        match self.sessions.lock().unwrap().entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.local, key.remote))?,
            Entry::Vacant(entry) => {
                // Add a scope so that the lock is freed asap
                let network_id = {
                    *self
                        .network_for_ip
                        .lock()
                        .unwrap()
                        .get(&remote.into_inner())
                        .unwrap()
                };
                NetworkId::set(&mut participants, network_id);
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
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let local = LocalAddress::try_from(&participants).unwrap();
        match self.listen_bindings.lock().unwrap().entry(local) {
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
        mut context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let header = Ipv4Header::from_bytes(message.iter())?;
        let remote = RemoteAddress::from(header.source);
        let local = LocalAddress::from(header.destination);
        let identifier = SessionId { local, remote };
        local.apply(&mut context.info);
        remote.apply(&mut context.info);
        let message = message.slice(header.ihl as usize * 4..);
        let mut session = match self.sessions.lock().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => match self.listen_bindings.lock().unwrap().get(&local) {
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

    fn start(
        &mut self,
        _context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
