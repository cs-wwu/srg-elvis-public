//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    internet::NetworkHandle,
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::tap::Tap,
    session::SharedSession,
    Control, Protocol, Session,
};
use std::{error::Error, sync::Arc};

mod ipv4_parsing;
use dashmap::{mapref::entry::Entry, DashMap};
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_misc;
use ipv4_misc::Ipv4Error;
pub use ipv4_misc::{LocalAddress, RemoteAddress};

mod ipv4_session;
use ipv4_session::{Ipv4Session, SessionId};

use super::tap::NetworkId;

pub type IpToNetwork = DashMap<Ipv4Address, NetworkHandle>;

/// An implementation of the Internet Protocol.
#[derive(Clone)]
pub struct Ipv4 {
    listen_bindings: DashMap<LocalAddress, ProtocolId>,
    sessions: DashMap<SessionId, Arc<Ipv4Session>>,
    ip_to_network: IpToNetwork,
}

impl Ipv4 {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(4);

    /// Creates a new instance of the protocol.
    pub fn new(network_for_ip: IpToNetwork) -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
            ip_to_network: network_for_ip,
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared(network_for_ip: IpToNetwork) -> Arc<Self> {
        Arc::new(Self::new(network_for_ip))
    }
}

// TODO(hardint): Add a static IP lookup table in the constructor so that
// messages can be sent to the correct network

impl Protocol for Ipv4 {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let local = LocalAddress::try_from(&participants).unwrap();
        let remote = RemoteAddress::try_from(&participants).unwrap();
        let key = SessionId { local, remote };
        match self.sessions.entry(key) {
            Entry::Occupied(_) => Err(Ipv4Error::SessionExists(key.local, key.remote))?,
            Entry::Vacant(entry) => {
                let network_id = { *self.ip_to_network.get(&remote.into_inner()).unwrap() };
                let tap_session = context.protocol(Tap::ID).expect("No such protocol").open(
                    Self::ID,
                    participants,
                    context,
                )?;
                let session = Arc::new(Ipv4Session::new(
                    tap_session,
                    upstream,
                    key,
                    network_id.into_inner().into(),
                ));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
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
            .listen(Self::ID, participants, context)
    }

    fn demux(
        self: Arc<Self>,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
    ) -> Result<(), Box<dyn Error>> {
        let header = Ipv4Header::from_bytes(message.iter())?;
        let remote = RemoteAddress::from(header.source);
        let local = LocalAddress::from(header.destination);
        let identifier = SessionId { local, remote };
        local.apply(&mut context.info);
        remote.apply(&mut context.info);
        message.slice(header.ihl as usize * 4..);
        let session = match self.sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => match self.listen_bindings.get(&local) {
                Some(binding) => {
                    let network = NetworkId::try_from(&context.info)?;
                    let session = Arc::new(Ipv4Session::new(caller, *binding, identifier, network));
                    entry.insert(session.clone());
                    session
                }
                None => Err(Ipv4Error::MissingListenBinding(local))?,
            },
        };
        session.receive(message, context)?;
        Ok(())
    }
}
