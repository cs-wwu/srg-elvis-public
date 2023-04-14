//! An implementation of the [Transmission Control
//! Protocol](https://www.rfc-editor.org/rfc/rfc9293.html).

use self::{
    tcb::{segment_arrives_closed, ListenResult, Segment, Tcb},
    tcp_parsing::TcpHeader,
    tcp_session::TcpSession,
};
use super::{pci::Pci, utility::Socket, Ipv4};
use crate::{
    control::{ControlError, Key, Primitive},
    protocol::{DemuxError, ListenError, OpenError, QueryError, StartError},
    protocols::tcp::tcb::segment_arrives_listen,
    session::SharedSession,
    Control, FxDashMap, Id, Message, Protocol, ProtocolMap,
};
use dashmap::mapref::entry::Entry;
use std::sync::Arc;

mod tcb;
mod tcp_parsing;
mod tcp_session;

// Problem: TCP packets don't use MAC addresses

/// Implements the Transmission Control Protocol. See the module-level
/// documentation for more details.
#[derive(Default)]
pub struct Tcp {
    /// A record of which protocol requested to listen for connections on
    /// particular sockets.
    listen_bindings: FxDashMap<Socket, Id>,
    /// A lookup table for sessions based on their endpoints.
    sessions: FxDashMap<ConnectionId, Arc<TcpSession>>,
}

impl Tcp {
    /// The simulation-unique ID for TCP.
    pub const ID: Id = Id::new(6);

    /// Creates a new TCP protocol
    pub fn new() -> Self {
        Self::default()
    }

    /// Converts the TCP into a shared protocol.
    pub fn shared(self) -> Arc<Self> {
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
    fn id(&self) -> Id {
        Self::ID
    }

    fn open(
        &self,
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
                    .query(Pci::MTU_QUERY_KEY)
                    .map_err(|_| OpenError::Other)?
                    .ok_u32()
                    .map_err(|_| OpenError::Other)?;
                let session = TcpSession::new(
                    Tcb::open(session_id, rand::random(), mtu),
                    protocols
                        .protocol(upstream)
                        .ok_or(OpenError::MissingProtocol(upstream))?,
                    downstream,
                    protocols,
                );
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn listen(
        &self,
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
        &self,
        mut message: Message,
        caller: SharedSession,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Extract information from the context
        let local_address = Ipv4::get_local_address(&control).unwrap();
        let remote_address = Ipv4::get_remote_address(&control).unwrap();

        // Parse the header
        let header =
            TcpHeader::from_bytes(message.iter(), message.len(), remote_address, local_address)
                .map_err(|_| DemuxError::Header)?;
        message.remove_front(20);

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
        Tcp::set_local_port(local.port, &mut control);
        Tcp::set_remote_port(remote.port, &mut control);

        let segment = Segment::new(header, message);
        match self.sessions.entry(connection_id) {
            Entry::Occupied(entry) => {
                // TODO(hardint): Do something with the result
                let _ = entry.get().receive(segment);
            }

            Entry::Vacant(session_entry) => {
                match self.listen_bindings.entry(local) {
                    Entry::Occupied(listen_entry) => {
                        // TODO(hardint): Incomplete. See 3.10.7.2 for handling
                        // of segments in LISTEN state.

                        // If we have a listen binding, create the session and
                        // save it
                        let mtu = caller
                            .query(Pci::MTU_QUERY_KEY)
                            .map_err(|_| DemuxError::Other)?
                            .ok_u32()
                            .map_err(|_| DemuxError::Other)?;
                        let listen_result = segment_arrives_listen(
                            segment,
                            local.address,
                            remote.address,
                            rand::random(),
                            mtu,
                        );
                        if let Some(listen_result) = listen_result {
                            match listen_result {
                                ListenResult::Response(response) => {
                                    caller.send(
                                        Message::new(response.serialize()),
                                        control,
                                        protocols,
                                    )?;
                                }
                                ListenResult::Tcb(tcb) => {
                                    let upstream = *listen_entry.get();
                                    let session = TcpSession::new(
                                        tcb,
                                        protocols
                                            .protocol(upstream)
                                            .ok_or(OpenError::MissingProtocol(upstream))?,
                                        caller,
                                        protocols,
                                    );
                                    session_entry.insert(session);
                                }
                            }
                        }
                    }

                    Entry::Vacant(_) => {
                        if let Some(response) = segment_arrives_closed(
                            segment.header,
                            segment.text.len() as u32,
                            local.address,
                            remote.address,
                        ) {
                            caller.send(Message::new(response.serialize()), control, protocols)?;
                        }
                        Err(DemuxError::MissingSession)?
                    }
                }
            }
        }
        Ok(())
    }

    fn start(&self, _protocols: ProtocolMap) -> Result<(), StartError> {
        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        eprintln!("No such key on TCP");
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
