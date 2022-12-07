//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    control::{Key, Primitive},
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::ipv4::{Ipv4, LocalAddress, RemoteAddress},
    session::SharedSession,
    Control, Protocol, Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc::Sender, Barrier};
use tracing::error;

mod udp_misc;
use udp_misc::UdpError;
pub use udp_misc::{LocalPort, RemotePort};

mod udp_session;
use udp_session::{SessionId, UdpSession};

mod udp_parsing;
use self::udp_parsing::UdpHeader;

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: DashMap<ListenId, ProtocolId>,
    sessions: DashMap<SessionId, Arc<UdpSession>>,
}

impl Udp {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(17);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Protocol for Udp {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        // Identify the session based on the participants. If any of the
        // identifying information we need is not provided, that is a bug in one
        // of the higher-up protocols and we should crash. Therefore, unwrapping
        // is appropriate here.
        let identifier = SessionId {
            local_port: LocalPort::try_from(&participants).unwrap(),
            remote_port: RemotePort::try_from(&participants).unwrap(),
            local_address: LocalAddress::try_from(&participants).unwrap(),
            remote_address: RemoteAddress::try_from(&participants).unwrap(),
        };
        match self.sessions.entry(identifier) {
            Entry::Occupied(_) => Err(UdpError::SessionExists)?,
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = context.protocol(Ipv4::ID).expect("No such protocol").open(
                    Self::ID,
                    participants,
                    context,
                )?;
                let session = Arc::new(UdpSession {
                    upstream,
                    downstream,
                    identifier,
                });
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
        // Add the listen binding. If any of the identifying information is
        // missing, that is a bug in the protocol that requested the listen and
        // we should crash. Unwrapping serves the purpose.
        let identifier = ListenId {
            port: LocalPort::try_from(&participants).unwrap(),
            address: LocalAddress::try_from(&participants).unwrap(),
        };
        self.listen_bindings.insert(identifier, upstream);
        // Ask lower-level protocols to add the binding as well
        context
            .protocol(Ipv4::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, context)
    }

    fn demux(
        self: Arc<Self>,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
    ) -> Result<(), Box<dyn Error>> {
        // Extract information from the context
        let local_address = LocalAddress::try_from(&context.info).unwrap();
        let remote_address = RemoteAddress::try_from(&context.info).unwrap();

        // Parse the header
        let header = UdpHeader::from_bytes_ipv4(
            message.iter(),
            remote_address.into(),
            local_address.into(),
        )?;
        message.slice(8..);

        // Use the context and the header information to identify the session
        let local_port = LocalPort::new(header.destination);
        let remote_port = RemotePort::new(header.source);
        let session_id = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };

        // Add the header information to the context
        local_port.apply(&mut context.info);
        remote_port.apply(&mut context.info);

        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let listen_id = ListenId {
                    address: local_address,
                    port: local_port,
                };
                match self.listen_bindings.entry(listen_id) {
                    Entry::Occupied(listen_entry) => {
                        // If we have a listen binding, create the session and
                        // save it
                        let session = Arc::new(UdpSession {
                            upstream: *listen_entry.get(),
                            downstream: caller,
                            identifier: session_id,
                        });
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => Err(UdpError::MissingSession)?,
                }
            }
        };
        session.receive(message, context)?;
        Ok(())
    }

    fn start(
        self: Arc<Self>,
        _context: Context,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, ()> {
        error!("No such key on UDP");
        Err(())
    }
}

/// An identifier for listen bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    /// The address being listened on
    address: LocalAddress,
    /// The port being listened on
    port: LocalPort,
}
