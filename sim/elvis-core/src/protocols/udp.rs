//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    control::{Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::ipv4::Ipv4,
    session::SharedSession,
    Control, FxDashMap, Protocol, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::sync::Arc;
use tokio::sync::Barrier;

mod udp_session;
use udp_session::{SessionId, UdpSession};

mod udp_parsing;
use self::udp_parsing::UdpHeader;

use super::{ipv4::Ipv4Address, utility::Socket};

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: FxDashMap<Socket, Id>,
    sessions: FxDashMap<SessionId, Arc<UdpSession>>,
}

impl Udp {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(17);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Protocol for Udp {
    fn id(&self) -> Id {
        Self::ID
    }

    #[tracing::instrument(name = "Udp::open", skip_all)]
    fn open(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        // Identify the session based on the participants. If any of the
        // identifying information we need is not provided, that is a bug in one
        // of the higher-up protocols and we should crash. Therefore, unwrapping
        // is appropriate here.
        let identifier = SessionId::new(
            Socket::new(
                participants.local.address.ok_or_else(|| {
                    tracing::error!("Missing local address on context");
                    OpenError::MissingContext
                })?,
                participants.remote.port.ok_or_else(|| {
                    tracing::error!("Missing local port on context");
                    OpenError::MissingContext
                })?,
            ),
            Socket::new(
                participants.remote.address.ok_or_else(|| {
                    tracing::error!("Missing remote address on context");
                    OpenError::MissingContext
                })?,
                participants.remote.port.ok_or_else(|| {
                    tracing::error!("Missing remote port on context");
                    OpenError::MissingContext
                })?,
            ),
        );
        match self.sessions.entry(identifier) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing)?
            }
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = protocols
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols)?;
                let session = Arc::new(UdpSession {
                    upstream,
                    downstream,
                    id: identifier,
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    #[tracing::instrument(name = "Udp::listen", skip_all)]
    fn listen(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        // Add the listen binding. If any of the identifying information is
        // missing, that is a bug in the protocol that requested the listen and
        // we should crash. Unwrapping serves the purpose.
        let identifier = Socket {
            port: participants.local.port.ok_or_else(|| {
                tracing::error!("Missing local port on context");
                ListenError::MissingContext
            })?,
            address: participants.local.address.ok_or_else(|| {
                tracing::error!("Missing local address on context");
                ListenError::MissingContext
            })?,
        };
        self.listen_bindings.insert(identifier, upstream);
        // Ask lower-level protocols to add the binding as well
        protocols
            .protocol(Ipv4::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)
    }

    #[tracing::instrument(name = "Udp::demux", skip_all)]
    fn demux(
        &self,
        mut message: Message,
        caller: SharedSession,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Extract information from the context
        let local_address = control.local.address.ok_or_else(|| {
            tracing::error!("Missing local address on context");
            DemuxError::MissingContext
        })?;
        let remote_address = control.remote.address.ok_or_else(|| {
            tracing::error!("Missing remote address on context");
            DemuxError::MissingContext
        })?;
        // Parse the header
        let header = match UdpHeader::from_bytes_ipv4(
            message.iter(),
            message.len(),
            remote_address,
            local_address,
        ) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        message.remove_front(8);

        // Use the context and the header information to identify the session
        let session_id = SessionId::new(
            Socket::new(local_address, header.destination),
            Socket::new(remote_address, header.source),
        );

        // Add the header information to the context
        control.local.port = Some(session_id.local.port);
        control.remote.port = Some(session_id.remote.port);
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => entry.get().clone(),

            Entry::Vacant(session_entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let listen_id = Socket {
                    address: local_address,
                    port: session_id.local.port,
                };
                let binding = match self.listen_bindings.get(&listen_id) {
                    Some(listen_entry) => listen_entry,
                    None => {
                        // If we don't have a normal listen binding, check for
                        // a 0.0.0.0 binding
                        let any_listen_id = Socket {
                            address: Ipv4Address::CURRENT_NETWORK,
                            port: session_id.local.port,
                        };
                        match self.listen_bindings.get(&any_listen_id) {
                            Some(any_listen_entry) => any_listen_entry,

                            None => {
                                tracing::error!(
                                    "Tried to demux with a missing session and no listen bindings"
                                );
                                Err(DemuxError::MissingSession)?
                            }
                        }
                    }
                };
                let session = Arc::new(UdpSession {
                    upstream: *binding,
                    downstream: caller,
                    id: session_id,
                });
                session_entry.insert(session.clone());
                session
            }
        };
        session.receive(message, control, protocols)?;
        Ok(())
    }

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
