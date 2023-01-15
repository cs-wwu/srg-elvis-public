//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    message::Message,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::ipv4::Ipv4,
    session::SharedSession,
    Control, Protocol,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

mod udp_session;
use udp_session::{SessionId, UdpSession};

mod udp_parsing;
use self::udp_parsing::UdpHeader;

use super::utility::Socket;

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: DashMap<Socket, Id>,
    sessions: DashMap<SessionId, Arc<UdpSession>>,
}

impl Udp {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(17);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set_local_port(port: u16, control: &mut Control) {
        control.insert((Self::ID, 0), port);
    }

    pub fn get_local_port(control: &Control) -> Result<u16, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u16()?)
    }

    pub fn set_remote_port(port: u16, control: &mut Control) {
        control.insert((Self::ID, 1), port);
    }

    pub fn get_remote_port(control: &Control) -> Result<u16, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u16()?)
    }
}

impl Protocol for Udp {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    #[tracing::instrument(name = "Udp::open", skip_all)]
    fn open(
        self: Arc<Self>,
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
                Ipv4::get_local_address(&participants).map_err(|_| {
                    tracing::error!("Missing local address on context");
                    OpenError::MissingContext
                })?,
                Self::get_local_port(&participants).map_err(|_| {
                    tracing::error!("Missing local port on context");
                    OpenError::MissingContext
                })?,
            ),
            Socket::new(
                Ipv4::get_remote_address(&participants).map_err(|_| {
                    tracing::error!("Missing remote address on context");
                    OpenError::MissingContext
                })?,
                Self::get_remote_port(&participants).map_err(|_| {
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
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        // Add the listen binding. If any of the identifying information is
        // missing, that is a bug in the protocol that requested the listen and
        // we should crash. Unwrapping serves the purpose.
        let identifier = Socket {
            port: Self::get_local_port(&participants).map_err(|_| {
                tracing::error!("Missing local port on context");
                ListenError::MissingContext
            })?,
            address: Ipv4::get_local_address(&participants).map_err(|_| {
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
        self: Arc<Self>,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
    ) -> Result<(), DemuxError> {
        // Extract information from the context
        let local_address = Ipv4::get_local_address(&context.control).map_err(|_| {
            tracing::error!("Missing local address on context");
            DemuxError::MissingContext
        })?;
        let remote_address = Ipv4::get_remote_address(&context.control).map_err(|_| {
            tracing::error!("Missing remote address on context");
            DemuxError::MissingContext
        })?;

        // Parse the header
        let header = match UdpHeader::from_bytes_ipv4(message.iter(), remote_address, local_address)
        {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        message.slice(8..);

        // Use the context and the header information to identify the session
        let session_id = SessionId::new(
            Socket::new(local_address, header.destination),
            Socket::new(remote_address, header.source),
        );

        // Add the header information to the context
        Self::set_local_port(session_id.local.port, &mut context.control);
        Self::set_remote_port(session_id.remote.port, &mut context.control);

        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let listen_id = Socket {
                    address: local_address,
                    port: session_id.local.port,
                };
                match self.listen_bindings.entry(listen_id) {
                    Entry::Occupied(listen_entry) => {
                        // If we have a listen binding, create the session and
                        // save it
                        let session = Arc::new(UdpSession {
                            upstream: *listen_entry.get(),
                            downstream: caller,
                            id: session_id,
                        });
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => {
                        tracing::error!(
                            "Tried to demux with a missing session and no listen bindings"
                        );
                        Err(DemuxError::MissingSession)?
                    }
                }
            }
        };
        session.receive(message, context)?;
        Ok(())
    }

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }
}
