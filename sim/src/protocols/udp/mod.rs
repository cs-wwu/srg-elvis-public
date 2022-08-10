//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    core::{message::Message, Control, Protocol, ProtocolContext, ProtocolId, SharedSession},
    protocols::ipv4::{Ipv4, LocalAddress, RemoteAddress},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;

mod udp_misc;
use udp_misc::UdpError;
pub use udp_misc::{LocalPort, RemotePort};

mod udp_session;
use udp_session::{SessionId, UdpSession};

use self::udp_parsing::UdpHeader;

mod udp_parsing;

type ArcMap<K, V> = Arc<Mutex<HashMap<K, V>>>;

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: ArcMap<ListenId, ProtocolId>,
    sessions: ArcMap<SessionId, SharedSession>,
}

impl Udp {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(17);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }
}

impl Protocol for Udp {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open(
        &self,
        upstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let identifier = SessionId {
            local_port: LocalPort::try_from(&participants).unwrap(),
            remote_port: RemotePort::try_from(&participants).unwrap(),
            local_address: LocalAddress::try_from(&participants).unwrap(),
            remote_address: RemoteAddress::try_from(&participants).unwrap(),
        };
        match self.sessions.lock().unwrap().entry(identifier) {
            Entry::Occupied(_) => Err(UdpError::SessionExists)?,
            Entry::Vacant(entry) => {
                let downstream = context
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .lock()
                    .unwrap()
                    .open(Self::ID, participants, context)?;
                let session = SharedSession::new(UdpSession {
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
        &self,
        upstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let identifier = ListenId {
            port: LocalPort::try_from(&participants).unwrap(),
            address: LocalAddress::try_from(&participants).unwrap(),
        };

        // Scope so that the lock is freed asap
        {
            self.listen_bindings
                .lock()
                .unwrap()
                .insert(identifier, upstream);
        }

        context
            .protocol(Ipv4::ID)
            .expect("No such protocol")
            .lock()
            .unwrap()
            .listen(Self::ID, participants, context)
    }

    fn demux(&self, message: Message, mut context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        let local_address = LocalAddress::try_from(&context.info).unwrap();
        let remote_address = RemoteAddress::try_from(&context.info).unwrap();
        let header = UdpHeader::from_bytes_ipv4(
            message.iter(),
            remote_address.into(),
            local_address.into(),
        )?;
        let local_port = LocalPort::new(header.destination);
        let remote_port = RemotePort::new(header.source);
        let session_id = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        local_port.apply(&mut context.info);
        remote_port.apply(&mut context.info);
        let message = message.slice(8..);
        let mut session = match self.sessions.lock().unwrap().entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                let listen_id = ListenId {
                    address: local_address,
                    port: local_port,
                };
                match self.listen_bindings.lock().unwrap().entry(listen_id) {
                    Entry::Occupied(listen_entry) => {
                        let session = SharedSession::new(UdpSession {
                            upstream: *listen_entry.get(),
                            downstream: context.current_session().expect("No current session"),
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
        &self,
        _context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    address: LocalAddress,
    port: LocalPort,
}
