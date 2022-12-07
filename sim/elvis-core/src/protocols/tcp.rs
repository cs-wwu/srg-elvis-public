use self::{
    tcp_parsing::TcpHeader,
    tcp_session::{SessionId, Socket, TcpSession},
};
use super::{
    ipv4::{LocalAddress, RemoteAddress},
    Ipv4,
};
use crate::{
    control::{
        self,
        value::{from_impls, make_key},
        Key, Primitive,
    },
    protocol::{Context, ProtocolId},
    session::SharedSession,
    Control, Message, Protocol, Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};
use thiserror::Error as ThisError;
use tokio::sync::{mpsc::Sender, Barrier};

mod tcp_parsing;
mod tcp_session;

pub struct Tcp {
    listen_bindings: DashMap<Socket, ProtocolId>,
    sessions: DashMap<SessionId, Arc<TcpSession>>,
}

impl Tcp {
    pub const ID: ProtocolId = ProtocolId::new(6);
}

impl Protocol for Tcp {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        _upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        // Identify the session based on the participants. If any of the
        // identifying information we need is not provided, that is a bug in one
        // of the higher-up protocols and we should crash. Therefore, unwrapping
        // is appropriate here.

        let local = Socket {
            address: LocalAddress::try_from(&participants).unwrap().into(),
            port: LocalPort::try_from(&participants).unwrap().into(),
        };

        let remote = Socket {
            address: RemoteAddress::try_from(&participants).unwrap().into(),
            port: RemotePort::try_from(&participants).unwrap().into(),
        };

        let session_id = SessionId { local, remote };
        match self.sessions.entry(session_id) {
            Entry::Occupied(_) => Err(TcpError::SessionExists)?,
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = context.protocol(Ipv4::ID).expect("No such protocol").open(
                    Self::ID,
                    participants,
                    context,
                )?;
                let session = Arc::new(TcpSession::open(session_id, downstream));
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
        let socket = Socket {
            port: LocalPort::try_from(&participants).unwrap().into(),
            address: LocalAddress::try_from(&participants).unwrap().into(),
        };
        self.listen_bindings.insert(socket, upstream);
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
        let header =
            TcpHeader::from_bytes(message.iter(), remote_address.into(), local_address.into())?;
        message.slice(20..);

        let local = Socket {
            address: local_address.into(),
            port: header.dst_port,
        };

        let remote = Socket {
            address: remote_address.into(),
            port: header.src_port,
        };

        // Use the context and the header information to identify the session
        let session_id = SessionId { local, remote };

        // Add the header information to the context
        LocalPort::new(local.port).apply(&mut context.info);
        RemotePort::new(remote.port).apply(&mut context.info);

        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                match self.listen_bindings.entry(local) {
                    Entry::Occupied(_listen_entry) => {
                        // If we have a listen binding, create the session and
                        // save it
                        let session = Arc::new(TcpSession::open(session_id, caller));
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => Err(TcpError::MissingSession)?,
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

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, Box<dyn Error>> {
        panic!("Nothing to query on the TCP protocol")
    }
}

const LOCAL_PORT_KEY: Key = make_key("TCP Local Port");
/// A [`control::Value`] for the local port number.
pub type LocalPort = control::Value<LOCAL_PORT_KEY, u16>;
from_impls!(LocalPort, u16);

const REMOTE_PORT_KEY: Key = make_key("TCP Remote Port");
/// A [`control::Value`] for the remote port number.
pub type RemotePort = control::Value<REMOTE_PORT_KEY, u16>;
from_impls!(RemotePort, u16);

#[derive(Debug, ThisError)]
pub enum TcpError {
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
    #[error("Too few bytes to constitute a TCP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    InvalidChecksum { actual: u16, expected: u16 },
    #[error("Data offset was different from that expected for a simple header")]
    UnexpectedOptions,
    #[error("The TCP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}
