//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::ipv4::Ipv4,
    session::SharedSession,
    Control, FxDashMap, Protocol, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

mod udp_session;
use udp_session::UdpSession;

mod udp_parsing;
pub use udp_parsing::UdpHeader;

use super::{
    ipv4::{self, ipv4_parsing::Ipv4Header, Ipv4Address},
    utility::{Endpoint, Endpoints},
};

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: FxDashMap<Endpoint, TypeId>,
    sessions: FxDashMap<Endpoints, Arc<UdpSession>>,
}

impl Udp {
    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn open(
        &self,
        upstream: TypeId,
        sockets: Endpoints,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        match self.sessions.entry(sockets) {
            Entry::Occupied(_) => {
                tracing::error!("Tried to create an existing session");
                Err(OpenError::Existing(sockets))
            }
            Entry::Vacant(entry) => {
                // Create the session and save it
                let downstream = protocols.protocol::<Ipv4>().unwrap().open(
                    TypeId::of::<Self>(),
                    sockets.into(),
                    protocols,
                )?;
                let session = Arc::new(UdpSession {
                    upstream,
                    downstream,
                    sockets,
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    pub fn listen(
        &self,
        upstream: TypeId,
        socket: Endpoint,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        match self.listen_bindings.entry(socket) {
            Entry::Occupied(_) => return Err(ListenError::Existing(socket)),
            Entry::Vacant(entry) => {
                let _ = entry.insert(upstream);
            }
        }
        // Ask lower-level protocols to add the binding as well
        protocols
            .protocol::<Ipv4>()
            .expect("No such protocol")
            .listen(TypeId::of::<Self>(), socket.address)?;
        Ok(())
    }
}

impl Protocol for Udp {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn demux(
        &self,
        mut message: Message,
        caller: SharedSession,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let ipv4_header = *control.get::<Ipv4Header>().unwrap();
        // Parse the header
        let udp_header = match UdpHeader::from_bytes_ipv4(
            message.iter(),
            message.len(),
            ipv4_header.source,
            ipv4_header.destination,
        ) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        message.remove_front(8);
        control.insert(udp_header);

        // Use the context and the header information to identify the session
        let endpoints = Endpoints::new(
            Endpoint::new(ipv4_header.destination, udp_header.destination),
            Endpoint::new(ipv4_header.source, udp_header.source),
        );

        let session = match self.sessions.entry(endpoints) {
            Entry::Occupied(entry) => entry.get().clone(),

            Entry::Vacant(session_entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let binding = match self.listen_bindings.get(&endpoints.local) {
                    Some(listen_entry) => listen_entry,
                    None => {
                        // If we don't have a normal listen binding, check for
                        // a 0.0.0.0 binding
                        let any_listen_id = Endpoint {
                            address: Ipv4Address::CURRENT_NETWORK,
                            port: endpoints.local.port,
                        };
                        match self.listen_bindings.get(&any_listen_id) {
                            Some(any_listen_entry) => any_listen_entry,

                            None => {
                                tracing::error!(
                                    "Tried to demux with a missing session and no listen bindings"
                                );
                                Err(DemuxError::MissingSession)?
                            }
                        }
                    }
                };
                let session = Arc::new(UdpSession {
                    upstream: *binding,
                    downstream: caller,
                    sockets: endpoints,
                });
                session_entry.insert(session.clone());
                session
            }
        };
        session.receive(message, control, protocols)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("The socket pair already has an associated session: {0:?}")]
    Existing(Endpoints),
    #[error("{0}")]
    Ipv4(#[from] ipv4::OpenError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListenError {
    #[error("The socket already has a listen binding: {0:?}")]
    Existing(Endpoint),
    #[error("{0}")]
    Ipv4(#[from] ipv4::ListenError),
}
