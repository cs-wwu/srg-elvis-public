use self::{
    tcp_parsing::{TcpHeader, TcpHeaderBuilder},
    tcp_session::{SessionId, Socket, TcpSession},
};
use super::{ipv4::Ipv4Address, Ipv4};
use crate::{
    control::{ControlError, Key, Primitive},
    protocol::{
        Context, DemuxError, ListenError, OpenError, ProtocolId, QueryError, SharedProtocol,
        StartError,
    },
    session::SharedSession,
    Control, Message, Protocol, ProtocolMap,
};
use dashmap::{mapref::entry::Entry, DashMap};
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::Sender, Barrier};

mod tcp_parsing;
mod tcp_session;

#[derive(Default)]
pub struct Tcp {
    listen_bindings: DashMap<Socket, ProtocolId>,
    sessions: DashMap<SessionId, Arc<TcpSession>>,
    iss_seed: Arc<Mutex<Iss>>,
}

impl Tcp {
    pub const ID: ProtocolId = ProtocolId::new(6);

    pub fn new(iss: Iss) -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
            iss_seed: Arc::new(Mutex::new(iss)),
        }
    }

    pub fn new_shared(iss: Iss) -> SharedProtocol {
        Arc::new(Self::new(iss))
    }

    fn next_iss(self: Arc<Self>) -> Iss {
        let mut lock = self.iss_seed.lock().unwrap();
        match *lock {
            Iss::Random => Iss::Random,
            Iss::FromSeed(c) => {
                let out = *lock;
                *lock = Iss::FromSeed(c + 1);
                out
            }
        }
    }

    // See 3.10.7.1 for handling of segments in CLOSED state
    fn respond_to_segment_from_closed_state(
        &self,
        header: TcpHeader,
        seg_len: u32,
        local_address: Ipv4Address,
        remote_address: Ipv4Address,
        session: SharedSession,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        if header.control.rst() {
            // Discard reset segments
            return Ok(());
        }

        let id = SessionId::new(
            Socket::new(local_address, header.dst_port),
            Socket::new(remote_address, header.src_port),
        );

        let response = if header.control.ack() {
            TcpHeaderBuilder::new(id, header.acknowledgement, 0)
                .rst()
                .build([].into_iter())
        } else {
            TcpHeaderBuilder::new(id, 0, 0)
                .ack(header.sequence + seg_len)
                .rst()
                .build([].into_iter())
        };

        let response = Message::new(response.map_err(|_| DemuxError::Header)?);
        let context = Context::new(protocols);
        session.send(response, context)?;
        Ok(())
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

impl Protocol for Tcp {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        // Identify the session based on the participants. If any of the
        // identifying information we need is not provided, that is a bug in one
        // of the higher-up protocols and we should crash. Therefore, unwrapping
        // is appropriate here.

        let local = Socket {
            address: Ipv4::get_local_address(&participants).unwrap(),
            port: Self::get_local_port(&participants).unwrap(),
        };

        let remote = Socket {
            address: Ipv4::get_remote_address(&participants).unwrap(),
            port: Self::get_remote_port(&participants).unwrap(),
        };

        let session_id = SessionId {
            src: local,
            dst: remote,
        };
        match self.clone().sessions.entry(session_id) {
            Entry::Occupied(_) => Err(OpenError::Existing)?,
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = protocols
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols.clone())?;
                let session =
                    TcpSession::open(session_id, upstream, downstream, self.next_iss(), protocols)?;
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        // Add the listen binding. If any of the identifying information is
        // missing, that is a bug in the protocol that requested the listen and
        // we should crash. Unwrapping serves the purpose.
        let socket = Socket {
            port: Self::get_local_port(&participants).unwrap(),
            address: Ipv4::get_local_address(&participants).unwrap(),
        };
        self.listen_bindings.insert(socket, upstream);
        // Ask lower-level protocols to add the binding as well
        protocols
            .protocol(Ipv4::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)
    }

    fn demux(
        self: Arc<Self>,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
    ) -> Result<(), DemuxError> {
        // Extract information from the context
        let local_address = Ipv4::get_local_address(&context.info).unwrap();
        let remote_address = Ipv4::get_remote_address(&context.info).unwrap();

        // Parse the header
        let header = TcpHeader::from_bytes(message.iter(), remote_address, local_address)
            .map_err(|_| DemuxError::Header)?;
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
        let session_id = SessionId {
            src: local,
            dst: remote,
        };

        // Add the header information to the context
        Tcp::set_local_port(local.port, &mut context.info);
        Tcp::set_remote_port(remote.port, &mut context.info);

        let session = match self.clone().sessions.entry(session_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(session_entry) => {
                match self.clone().listen_bindings.entry(local) {
                    Entry::Occupied(listen_entry) => {
                        // If we have a listen binding, create the session and
                        // save it
                        let session = TcpSession::open(
                            session_id,
                            *listen_entry.get(),
                            caller,
                            self.next_iss(),
                            context.protocols,
                        )?;
                        session_entry.insert(session.clone());
                        session
                    }
                    Entry::Vacant(_) => {
                        self.respond_to_segment_from_closed_state(
                            header,
                            header.len() + message.len() as u32,
                            local_address,
                            remote_address,
                            caller,
                            context.protocols,
                        )?;
                        Err(DemuxError::MissingSession)?
                    }
                }
            }
        };
        session
            .receive(message, header)
            .map_err(|_| DemuxError::Other)?;
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
        tracing::error!("No such key on TCP");
        Err(QueryError::NonexistentKey)
    }
}

/// The initial send sequence of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Iss {
    #[default]
    Random,
    FromSeed(u64),
}

impl From<Iss> for u32 {
    fn from(iss: Iss) -> Self {
        let mut rng = match iss {
            Iss::Random => SmallRng::from_entropy(),
            Iss::FromSeed(c) => SmallRng::seed_from_u64(c),
        };
        rng.next_u32()
    }
}
