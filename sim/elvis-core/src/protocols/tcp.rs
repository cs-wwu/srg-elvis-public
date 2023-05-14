//! An implementation of the [Transmission Control
//! Protocol](https://www.rfc-editor.org/rfc/rfc9293.html).

use self::{
    tcb::{segment_arrives_closed, ListenResult, Segment, Tcb},
    tcp_parsing::TcpHeader,
    tcp_session::TcpSession,
};
use super::{pci::pci_session::SessionInfo, utility::Socket, Ipv4, Pci};
use crate::{
    machine::ProtocolMap,
    protocol::{DemuxError, ListenError, OpenError, StartError},
    protocols::tcp::tcb::segment_arrives_listen,
    session::SharedSession,
    Control, FxDashMap, Message, Participants, Protocol, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

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
    listen_bindings: FxDashMap<Socket, TypeId>,
    /// A lookup table for sessions based on their endpoints.
    sessions: FxDashMap<ConnectionId, Arc<TcpSession>>,
}

impl Tcp {
    /// Creates a new TCP protocol
    pub fn new() -> Self {
        Self::default()
    }

    /// Converts the TCP into a shared protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Protocol for Tcp {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn open(
        &self,
        upstream: TypeId,
        participants: Participants,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        // Identify the session based on the participants. If any of the
        // identifying information we need is not provided, that is a bug in one
        // of the higher-up protocols and we should crash. Therefore, unwrapping
        // is appropriate here.

        let local = Socket {
            address: participants.local.address.unwrap(),
            port: participants.local.port.unwrap(),
        };

        let remote = Socket {
            address: participants.remote.address.unwrap(),
            port: participants.remote.port.unwrap(),
        };

        let session_id = ConnectionId { local, remote };

        match self.sessions.entry(session_id) {
            Entry::Occupied(_) => Err(OpenError::Existing),
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = protocols
                    .protocol::<Ipv4>()
                    .expect("No such protocol")
                    .open(TypeId::of::<Self>(), participants, protocols.clone())?;
                let pci_session_info = downstream
                    .info(TypeId::of::<Pci>())
                    .expect("Could not get PCI session info")
                    .downcast::<SessionInfo>()
                    .expect("Could not cast PCI session info");
                let session = TcpSession::new(
                    Tcb::open(session_id, rand::random(), pci_session_info.mtu),
                    protocols
                        .get(upstream)
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
        upstream: TypeId,
        participants: Participants,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        // Add the listen binding. If any of the identifying information is
        // missing, that is a bug in the protocol that requested the listen and
        // we should crash. Unwrapping serves the purpose.
        let socket = Socket {
            port: participants.local.port.unwrap(),
            address: participants.local.address.unwrap(),
        };
        self.listen_bindings.insert(socket, upstream);
        // Ask lower-level protocols to add the binding as well
        protocols
            .protocol::<Ipv4>()
            .expect("No such protocol")
            .listen(TypeId::of::<Self>(), participants, protocols)
    }

    fn demux(
        &self,
        mut message: Message,
        caller: SharedSession,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Extract information from the context
        let local_address = control.local.address.unwrap();
        let remote_address = control.remote.address.unwrap();

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
        control.local.port = Some(local.port);
        control.remote.port = Some(remote.port);

        let segment = Segment::new(header, message);
        match self.sessions.entry(connection_id) {
            Entry::Occupied(entry) => {
                entry.get().receive(segment);
            }

            Entry::Vacant(session_entry) => {
                match self.listen_bindings.entry(local) {
                    Entry::Occupied(listen_entry) => {
                        // TODO(hardint): Incomplete. See 3.10.7.2 for handling
                        // of segments in LISTEN state.

                        // If we have a listen binding, create the session and
                        // save it
                        let pci_session_info = caller
                            .info(TypeId::of::<Pci>())
                            .expect("No PCI session info")
                            .downcast::<SessionInfo>()
                            .expect("Could not cast PCI session info");
                        let listen_result = segment_arrives_listen(
                            segment,
                            local.address,
                            remote.address,
                            rand::random(),
                            pci_session_info.mtu,
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
                                            .get(upstream)
                                            .ok_or(OpenError::MissingProtocol(upstream))?,
                                        caller,
                                        protocols.clone(),
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

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
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
