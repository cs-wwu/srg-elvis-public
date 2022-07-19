//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    core::{
        message::Message, Control, ControlFlow, Protocol, ProtocolContext, ProtocolId,
        SharedSession,
    },
    protocols::ipv4::{Ipv4, LocalAddress, RemoteAddress},
};
use etherparse::UdpHeaderSlice;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod udp_misc;
use udp_misc::UdpError;
pub use udp_misc::{LocalPort, RemotePort};

mod udp_session;
use udp_session::{SessionId, UdpSession};

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: HashMap<ListenId, ProtocolId>,
    sessions: HashMap<SessionId, SharedSession>,
}

impl Udp {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::of::<Self>();

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }
}

impl Protocol for Udp {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
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
                let downstream = context
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .borrow_mut()
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
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let identifier = ListenId {
            port: LocalPort::try_from(&participants).unwrap(),
            address: LocalAddress::try_from(&participants).unwrap(),
        };
        self.listen_bindings.insert(identifier, upstream);

        context
            .protocol(Ipv4::ID)
            .expect("No such protocol")
            .borrow_mut()
            .listen(Self::ID, participants, context)
    }

    fn demux(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // Todo: Scuffed copy fest. Revise.
        let header_bytes: Vec<_> = message.iter().take(8).collect();
        let header = UdpHeaderSlice::from_slice(header_bytes.as_slice())?;
        let local_address = LocalAddress::try_from(&context.info).unwrap();
        let remote_address = RemoteAddress::try_from(&context.info).unwrap();
        let local_port = LocalPort::new(header.destination_port());
        let remote_port = RemotePort::new(header.source_port());
        let session_id = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        local_port.apply(&mut context.info);
        remote_port.apply(&mut context.info);
        let message = message.slice(8..);
        let mut session = match self.sessions.entry(session_id) {
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

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    address: LocalAddress,
    port: LocalPort,
}
