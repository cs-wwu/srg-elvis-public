//! An implementation of the [Transmission Control
//! Protocol](https://www.rfc-editor.org/rfc/rfc9293.html).

use self::{
    tcb::{segment_arrives_closed, ListenResult, Segment, Tcb},
    tcp_session::TcpSession,
};
use super::{
    ipv4::{self, ipv4_parsing::Ipv4Header, Ipv4Address},
    pci,
    utility::{Endpoint, Endpoints},
    Ipv4,
};
use crate::{
    protocol::{DemuxError, StartError},
    protocols::tcp::tcb::segment_arrives_listen,
    Control, FxDashMap, Machine, Message, Protocol, Session, Shutdown, internet::DoneSender,
};
use dashmap::mapref::entry::Entry;
use std::{any::TypeId, sync::Arc};

mod tcb;
mod tcp_parsing;
mod tcp_session;
pub use tcp_parsing::TcpHeader;

// Problem: TCP packets don't use MAC addresses

/// Implements the Transmission Control Protocol. See the module-level
/// documentation for more details.
#[derive(Default)]
pub struct Tcp {
    /// A record of which protocol requested to listen for connections on
    /// particular sockets.
    listen_bindings: FxDashMap<Endpoint, TypeId>,
    /// A lookup table for sessions based on their endpoints.
    sessions: FxDashMap<Endpoints, Arc<TcpSession>>,
}

impl Tcp {
    /// Creates a new TCP protocol
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(
        &self,
        upstream: TypeId,
        endpoints: Endpoints,
        machine: Arc<Machine>,
    ) -> Result<Arc<dyn Session>, OpenError> {
        match self.sessions.entry(endpoints) {
            Entry::Occupied(_) => Err(OpenError::Existing(endpoints)),
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = machine
                    .protocol::<Ipv4>()
                    .unwrap()
                    .open_and_listen(
                        TypeId::of::<Self>(),
                        endpoints.into(),
                        machine.clone(),
                        ipv4::ProtocolNumber::TCP,
                    )
                    .await?;
                let session = TcpSession::new(
                    Tcb::open(endpoints, rand::random(), downstream.pci_session().mtu()),
                    machine.get(upstream).unwrap(),
                    downstream,
                    machine,
                    endpoints,
                );
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    pub fn listen(
        &self,
        upstream: TypeId,
        endpoint: Endpoint,
        machine: Arc<Machine>,
    ) -> Result<(), ListenError> {
        self.listen_bindings.insert(endpoint, upstream);
        Ok(machine.protocol::<Ipv4>().unwrap().listen(
            TypeId::of::<Self>(),
            endpoint.address,
            machine,
            ipv4::ProtocolNumber::TCP,
        )?)
    }
}

#[async_trait::async_trait]
impl Protocol for Tcp {
    fn demux(
        &self,
        mut message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        let ipv4_header = control
            .get::<Ipv4Header>()
            .ok_or(DemuxError::MissingContext)?;

        // Parse the header
        let tcp_header = TcpHeader::from_bytes(
            message.iter(),
            message.len(),
            ipv4_header.source,
            ipv4_header.destination,
        )
        .map_err(|_| DemuxError::Header)?;
        message.remove_front(20);

        let endpoints = Endpoints {
            local: Endpoint {
                address: ipv4_header.destination,
                port: tcp_header.dst_port,
            },
            remote: Endpoint {
                address: ipv4_header.source,
                port: tcp_header.src_port,
            },
        };

        let segment = Segment::new(tcp_header, message);
        // TODO(hardint): Incomplete. See 3.10.7.2 for handling
        // of segments in LISTEN state.
        match self.sessions.entry(endpoints) {
            Entry::Occupied(entry) => entry.get().receive(segment),
            Entry::Vacant(session_entry) => {
                let binding = match self.listen_bindings.entry(endpoints.local) {
                    Entry::Occupied(listen_entry) => listen_entry,
                    Entry::Vacant(_) => {
                        let any_listen_id = Endpoint {
                            address: Ipv4Address::CURRENT_NETWORK,
                            port: endpoints.local.port,
                        };
                        match self.listen_bindings.entry(any_listen_id) {
                            Entry::Occupied(any_listen_entry) => any_listen_entry,
                            Entry::Vacant(_) => {
                                if let Some(response) = segment_arrives_closed(
                                    segment.header,
                                    segment.text.len() as u32,
                                    endpoints.local.address,
                                    endpoints.remote.address,
                                ) {
                                    caller
                                        .send(Message::new(response.serialize()), machine)
                                        .map_err(|_| DemuxError::Other)?;
                                }
                                return Err(DemuxError::MissingSession)?;
                            }
                        }
                    }
                };
                // If we have a listen binding, create the session and
                // save it
                let mtu = control.get::<pci::DemuxInfo>().unwrap().mtu;
                let listen_result = segment_arrives_listen(
                    segment,
                    endpoints.local.address,
                    endpoints.remote.address,
                    rand::random(),
                    mtu,
                );
                if let Some(listen_result) = listen_result {
                    match listen_result {
                        ListenResult::Response(response) => {
                            caller
                                .send(Message::new(response.serialize()), machine)
                                .map_err(|_| DemuxError::Other)?;
                        }
                        ListenResult::Tcb(tcb) => {
                            let upstream = *binding.get();
                            let session = TcpSession::new(
                                tcb,
                                machine
                                    .get(upstream)
                                    .ok_or(DemuxError::MissingProtocol(upstream))?,
                                caller,
                                machine.clone(),
                                endpoints,
                            );
                            session_entry.insert(session);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn start(
        &self,
        _shutdown: Shutdown,
        init_done: DoneSender,
        _machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        init_done.done();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("The socket pair already has an associated session: {0:?}")]
    Existing(Endpoints),
    #[error("{0}")]
    Ipv4(#[from] ipv4::OpenAndListenError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListenError {
    #[error("The socket already has a listen binding: {0:?}")]
    Existing(Endpoint),
    #[error("{0}")]
    Ipv4(#[from] ipv4::ListenError),
}
