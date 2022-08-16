//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    core::{
        message::Message,
        protocol::{Context, ProtocolId},
        session::SharedSession,
        Control, Protocol, Session,
    },
    protocols::ipv4::{Ipv4, LocalAddress, RemoteAddress},
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};

mod udp_misc;
use udp_misc::UdpError;
pub use udp_misc::{LocalPort, RemotePort};

mod udp_session;
use udp_session::{SessionId, UdpSession};

use self::udp_parsing::UdpHeader;

mod udp_parsing;

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
        let identifier = SessionId {
            local_port: LocalPort::try_from(&participants).unwrap(),
            remote_port: RemotePort::try_from(&participants).unwrap(),
            local_address: LocalAddress::try_from(&participants).unwrap(),
            remote_address: RemoteAddress::try_from(&participants).unwrap(),
        };
        match self.sessions.entry(identifier) {
            Entry::Occupied(_) => Err(UdpError::SessionExists)?,
            Entry::Vacant(entry) => {
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
        let identifier = ListenId {
            port: LocalPort::try_from(&participants).unwrap(),
            address: LocalAddress::try_from(&participants).unwrap(),
        };

        self.listen_bindings.insert(identifier, upstream);

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
        message.slice(8..);
        let session = match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                let listen_id = ListenId {
                    address: local_address,
                    port: local_port,
                };
                match self.listen_bindings.entry(listen_id) {
                    Entry::Occupied(listen_entry) => {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    address: LocalAddress,
    port: LocalPort,
}
