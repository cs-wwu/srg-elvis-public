//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use super::tap::NetworkId;
use crate::{
    control::{Key, Primitive},
    internet::NetworkHandle,
    message::Message,
    protocol::{Context, DemuxError, ProtocolId, QueryError},
    protocols::tap::Tap,
    session::SharedSession,
    Control, Protocol, Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

mod ipv4_parsing;
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_misc;
pub use ipv4_misc::{LocalAddress, RemoteAddress};

mod ipv4_session;
use ipv4_session::{Ipv4Session, SessionId};

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

    #[tracing::instrument(name = "Udp::open", skip_all)]
    fn open(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<SharedSession, ()> {
        let span = tracing::trace_span!("IPv4 open");
        let _enter = span.enter();
        // Extract identifying information from the participants list
        let local = LocalAddress::try_from(&participants).unwrap();
        let remote = RemoteAddress::try_from(&participants).unwrap();
        let key = SessionId { local, remote };
        match self.sessions.entry(key) {
            Entry::Occupied(_) => {
                tracing::error!(
                    "Attempting to create a session that already exists for {} -> {}",
                    key.local,
                    key.remote
                );
                Err(())?
            }
            Entry::Vacant(entry) => {
                // If the session does not exist, create it
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

    #[tracing::instrument(name = "Udp::listen", skip_all)]
    fn listen(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<(), ()> {
        let span = tracing::trace_span!("IPv4 listen");
        let _enter = span.enter();
        let local = LocalAddress::try_from(&participants).unwrap();
        match self.listen_bindings.entry(local) {
            Entry::Occupied(_) => {
                tracing::error!(
                    "Attempting to create a binding that already exists for local address {}",
                    local
                );
                Err(())?
            }
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

    #[tracing::instrument(name = "Udp::demux", skip_all)]
    fn demux(
        self: Arc<Self>,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
    ) -> Result<(), DemuxError> {
        let span = tracing::trace_span!("IPv4 demux");
        let _enter = span.enter();
        // Extract identifying information from the header and the context and
        // add header information to the context
        let header = match Ipv4Header::from_bytes(message.iter()) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        message.slice(header.ihl as usize * 4..);
        let remote = RemoteAddress::from(header.source);
        let local = LocalAddress::from(header.destination);
        let identifier = SessionId { local, remote };

        local.apply(&mut context.info);
        remote.apply(&mut context.info);

        let session = match self.sessions.entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => match self.listen_bindings.get(&local) {
                Some(binding) => {
                    // If the session does not exist but we have a listen
                    // binding for it, create the session
                    let network = match NetworkId::try_from(&context.info) {
                        Ok(network) => network,
                        Err(e) => {
                            tracing::error!("{}", e);
                            Err(DemuxError::MissingContext)?
                        }
                    };
                    let session = Arc::new(Ipv4Session::new(caller, *binding, identifier, network));
                    entry.insert(session.clone());
                    session
                }
                None => {
                    tracing::error!(
                        "Could not find a listen binding for the local address: {}",
                        local
                    );
                    Err(DemuxError::MissingSession)?
                }
            },
        };
        session.receive(message, context)?;
        Ok(())
    }

    fn start(
        self: Arc<Self>,
        _context: Context,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), ()> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
