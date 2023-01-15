use self::{tcp_parsing::TcpHeader, tcp_session::TcpSession};
use super::{utility::Socket, Ipv4};
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

mod tcb;
mod tcp_parsing;
mod tcp_session;

#[derive(Default)]
pub struct Tcp {
    listen_bindings: DashMap<Socket, ProtocolId>,
    sessions: DashMap<ConnectionId, Arc<TcpSession>>,
    iss: Arc<Mutex<IssGenerator>>,
}

impl Tcp {
    pub const ID: ProtocolId = ProtocolId::new(6);

    pub fn new(iss: IssGenerator) -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
            iss: Arc::new(Mutex::new(iss)),
        }
    }

    pub fn new_shared(iss: IssGenerator) -> SharedProtocol {
        Arc::new(Self::new(iss))
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
        _upstream: ProtocolId,
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

        let session_id = ConnectionId { local, remote };

        match self.sessions.entry(session_id) {
            Entry::Occupied(_) => Err(OpenError::Existing)?,
            Entry::Vacant(_entry) => {
                // Create the session and save it
                let _downstream = protocols
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols.clone())?;
                // TODO(hardint): Open and add session
                todo!()
            }
        }
        todo!()
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
        _caller: SharedSession,
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
            address: local_address,
            port: header.dst_port,
        };

        let remote = Socket {
            address: remote_address,
            port: header.src_port,
        };

        // Use the context and the header information to identify the session
        let connection_id = ConnectionId { local, remote };

        // Add the header information to the context
        Tcp::set_local_port(local.port, &mut context.info);
        Tcp::set_remote_port(remote.port, &mut context.info);

        let _session = match self.sessions.entry(connection_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                session
            }
            Entry::Vacant(_session_entry) => {
                match self.listen_bindings.entry(local) {
                    Entry::Occupied(_listen_entry) => {
                        // TODO(hardint): Incomplete. See 3.10.7.2 for handling
                        // of segments in LISTEN state.

                        // If we have a listen binding, create the session and
                        // save it
                        todo!()
                    }

                    Entry::Vacant(_) => {
                        todo!()
                    }
                }
            }
        };

        // TODO(hardint): Receive message
        todo!()
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
pub enum IssGenerator {
    #[default]
    Random,
    FromSeed(u64),
    Exact(u32),
}

impl IssGenerator {
    pub fn next_iss(&mut self) -> u32 {
        match self {
            Self::Random => SmallRng::from_entropy().next_u32(),
            Self::FromSeed(c) => {
                let out = SmallRng::seed_from_u64(*c).next_u32();
                *c += 1;
                out
            }
            Self::Exact(n) => *n,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ConnectionId {
    pub local: Socket,
    pub remote: Socket,
}

impl ConnectionId {
    pub fn new(local: Socket, remote: Socket) -> Self {
        Self { local, remote }
    }

    pub const fn reverse(self) -> Self {
        Self {
            local: self.remote,
            remote: self.local,
        }
    }
}
