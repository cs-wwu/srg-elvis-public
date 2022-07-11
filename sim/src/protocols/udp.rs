use crate::{
    core::{
        message::Message, Control, ControlFlow, NetworkLayer, Protocol, ProtocolContext,
        ProtocolId, SharedSession,
    },
    protocols::ipv4::{get_local_address, get_remote_address, Ipv4, Ipv4Address},
};
use etherparse::UdpHeaderSlice;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};

mod udp_misc;
pub use udp_misc::{get_local_port, get_remote_port, UdpError, LOCAL_PORT_KEY, REMOTE_PORT_KEY};

mod udp_session;
pub use udp_session::UdpSession;

use self::udp_session::SessionId;

#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: HashMap<ListenId, ProtocolId>,
    sessions: HashMap<SessionId, SharedSession>,
}

impl Udp {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Transport, 17);

    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }
}

impl Protocol for Udp {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let local_port = get_local_port(&participants);
        let remote_port = get_remote_port(&participants);
        let local_address = get_local_address(&participants);
        let remote_address = get_remote_address(&participants);
        let identifier = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        match self.sessions.entry(identifier) {
            Entry::Occupied(_) => Err(UdpError::SessionExists)?,
            Entry::Vacant(entry) => {
                let downstream = context
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .borrow_mut()
                    .open_active(Self::ID, participants, context)?;
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
        let port = get_local_port(&participants);
        let address = get_local_address(&participants);
        let identifier = ListenId { address, port };
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
        let local_address = get_local_address(&context.info);
        let remote_address = get_remote_address(&context.info);
        let local_port = header.destination_port();
        let remote_port = header.source_port();
        let session_id = SessionId {
            local_address,
            local_port,
            remote_address,
            remote_port,
        };
        context.info.insert(LOCAL_PORT_KEY, local_port);
        context.info.insert(REMOTE_PORT_KEY, remote_port);
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
                            downstream: context
                                .current_session
                                .clone()
                                .expect("No current session"),
                            identifier: session_id,
                        });
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => Err(UdpError::MissingSession)?,
                }
            }
        };
        session.recv(message, context)?;
        Ok(())
    }

    fn awake(&mut self, _context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ListenId {
    address: Ipv4Address,
    port: u16,
}
