//! An implementation of the [Transmission Control
//! Protocol](https://www.rfc-editor.org/rfc/rfc9293.html).

use self::{
    tcb::{handle_closed, ListenResult, Segment, SegmentArrivesResult, Tcb},
    tcp_parsing::TcpHeader,
    tcp_session::{ReceiveError, TcpSession},
};
use super::{utility::Socket, Ipv4, Pci};
use crate::{
    control::{ControlError, Key, Primitive},
    protocol::{
        Context, DemuxError, ListenError, OpenError, QueryError, SharedProtocol, StartError,
    },
    protocols::tcp::tcb::handle_listen,
    session::SharedSession,
    Control, Id, Message, Protocol, ProtocolMap,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::{mpsc::Sender, Barrier};

mod tcb;
mod tcp_parsing;
mod tcp_session;

/// Implements the Transmission Control Protocol. See the module-level
/// documentation for more details.
#[derive(Default)]
pub struct Tcp {
    /// A record of which protocol requested to listen for connections on
    /// particular sockets.
    listen_bindings: DashMap<Socket, Id>,
    /// A lookup table for sessions based on their endpoints.
    sessions: DashMap<ConnectionId, Arc<TcpSession>>,
}

impl Tcp {
    /// The simulation-unique ID for TCP.
    pub const ID: Id = Id::new(6);

    /// Creates a new TCP protocol
    pub fn new() -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
        }
    }

    /// Converts the TCP into a shared protocol.
    pub fn shared(self) -> SharedProtocol {
        Arc::new(self)
    }

    /// Set the local port number on a control.
    pub fn set_local_port(port: u16, control: &mut Control) {
        control.insert((Self::ID, 0), port);
    }

    /// Get the local port number from a control.
    pub fn get_local_port(control: &Control) -> Result<u16, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u16()?)
    }

    /// Set the remote port number on a control.
    pub fn set_remote_port(port: u16, control: &mut Control) {
        control.insert((Self::ID, 1), port);
    }

    /// Get the remote port number from a control.
    pub fn get_remote_port(control: &Control) -> Result<u16, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u16()?)
    }
}

impl Protocol for Tcp {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

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
            Entry::Occupied(_) => Err(OpenError::Existing),
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = protocols
                    .protocol(Ipv4::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols.clone())?;
                let mtu = downstream
                    .clone()
                    .query(Pci::MTU_QUERY_KEY)
                    .map_err(|_| OpenError::Other)?
                    .ok_u32()
                    .map_err(|_| OpenError::Other)?;
                let session = Arc::new(TcpSession::new(
                    RwLock::new(Tcb::open(session_id, rand::random(), mtu)),
                    upstream,
                    downstream,
                ));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        self: Arc<Self>,
        upstream: Id,
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
        let local_address = Ipv4::get_local_address(&context.control).unwrap();
        let remote_address = Ipv4::get_remote_address(&context.control).unwrap();

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
        Tcp::set_local_port(local.port, &mut context.control);
        Tcp::set_remote_port(remote.port, &mut context.control);

        let segment = Segment::new(header, message);
        match self.sessions.entry(connection_id) {
            Entry::Occupied(entry) => {
                let session = entry.get().clone();
                match session.receive(segment, context) {
                    Ok(receive_result) => {
                        if receive_result == SegmentArrivesResult::Close {
                            entry.remove_entry();
                        }
                    }
                    Err(e) => match e {
                        ReceiveError::Closing => {
                            tracing::error!("The TCP connection is already closing. Cannot demux.");
                            return Err(DemuxError::Other);
                        }
                        ReceiveError::Protocol(id) => return Err(DemuxError::MissingProtocol(id)),
                        ReceiveError::Demux(e) => Err(e)?,
                        ReceiveError::Send(e) => Err(e)?,
                    },
                }
            }

            Entry::Vacant(session_entry) => {
                match self.listen_bindings.entry(local) {
                    Entry::Occupied(listen_entry) => {
                        // TODO(hardint): Incomplete. See 3.10.7.2 for handling
                        // of segments in LISTEN state.

                        // If we have a listen binding, create the session and
                        // save it
                        let mtu = caller
                            .clone()
                            .query(Pci::MTU_QUERY_KEY)
                            .map_err(|_| DemuxError::Other)?
                            .ok_u32()
                            .map_err(|_| DemuxError::Other)?;
                        let listen_result = handle_listen(
                            segment,
                            local.address,
                            remote.address,
                            rand::random(),
                            mtu,
                        );
                        if let Some(listen_result) = listen_result {
                            match listen_result {
                                ListenResult::Response(response) => {
                                    caller.send(Message::new(response.serialize()), context)?;
                                }
                                ListenResult::Tcb(tcb) => {
                                    let session = Arc::new(TcpSession::new(
                                        RwLock::new(tcb),
                                        *listen_entry.get(),
                                        caller,
                                    ));
                                    session_entry.insert(session);
                                }
                            }
                        }
                    }

                    Entry::Vacant(_) => {
                        if let Some(response) = handle_closed(
                            segment.header,
                            segment.text.len() as u32,
                            local.address,
                            remote.address,
                        ) {
                            caller.send(Message::new(response.serialize()), context)?;
                        }
                        Err(DemuxError::MissingSession)?
                    }
                }
            }
        }
        Ok(())
    }

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
            loop {
                const SLEEP_DURATION: Duration = Duration::from_millis(33);
                tokio::time::sleep(SLEEP_DURATION).await;
                for session in self.sessions.iter_mut() {
                    session
                        .clone()
                        .advance_time(SLEEP_DURATION, protocols.clone());
                }
            }
        });
        Ok(())
    }

    fn query(self: Arc<Self>, _key: Key) -> Result<Primitive, QueryError> {
        tracing::error!("No such key on TCP");
        Err(QueryError::NonexistentKey)
    }
}

/// A pair of endpoints that uniquely identifies a TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ConnectionId {
    /// The local endpoint
    pub local: Socket,
    /// The remote endpoint
    pub remote: Socket,
}

impl ConnectionId {
    /// Create a new connection ID from a pair of endpoints
    pub fn new(local: Socket, remote: Socket) -> Self {
        Self { local, remote }
    }

    /// Get a matching connection ID for the remote TCP.
    pub const fn reverse(self) -> Self {
        Self {
            local: self.remote,
            remote: self.local,
        }
    }
}
