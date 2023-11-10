//! An implementation of the [User Datagram
//! Protocol](https://www.ietf.org/rfc/rfc768.txt).

use crate::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::ipv4::Ipv4,
    Control, FxDashMap, Machine, Protocol, Session, Shutdown,
};
use dashmap::mapref::entry::Entry;
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

mod udp_session;
use udp_session::UdpSession;

mod udp_parsing;
pub use udp_parsing::UdpHeader;

use super::{
    ipv4::{self, ipv4_parsing::Ipv4Header, Ipv4Address, Recipient},
    utility::{Endpoint, Endpoints},
};

/// An implementation of the User Datagram Protocol.
#[derive(Default, Clone)]
pub struct Udp {
    listen_bindings: FxDashMap<Endpoint, TypeId>,
}

impl Udp {
    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn open_and_listen(
        &self,
        upstream: TypeId,
        endpoints: Endpoints,
        machine: Arc<Machine>,
    ) -> Result<Arc<dyn Session>, OpenAndListenError> {
        self.listen(upstream, endpoints.local, machine.clone())?;

        Ok(self.open_for_sending(upstream, endpoints, machine).await?)
    }

    pub async fn open_for_sending(
        &self,
        upstream: TypeId,
        endpoints: Endpoints,
        machine: Arc<Machine>,
    ) -> Result<Arc<dyn Session>, OpenError> {
        let downstream = machine
            .protocol::<Ipv4>()
            .unwrap()
            .open_for_sending(TypeId::of::<Self>(), endpoints.into(), machine)
            .await?;

        let session = Arc::new(UdpSession {
            upstream,
            downstream,
            endpoints,
        });

        Ok(session)
    }

    pub fn listen(
        &self,
        upstream: TypeId,
        socket: Endpoint,
        machine: Arc<Machine>,
    ) -> Result<(), ListenError> {
        match self.listen_bindings.entry(socket) {
            Entry::Occupied(_) => return Err(ListenError::Existing(socket)),
            Entry::Vacant(entry) => {
                let _ = entry.insert(upstream);
            }
        }
        // Ask lower-level protocols to add the binding as well
        machine
            .protocol::<Ipv4>()
            .expect("No such protocol")
            .listen(
                TypeId::of::<Self>(),
                socket.address,
                machine,
                ipv4::ProtocolNumber::UDP,
            )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Protocol for Udp {
    fn demux(
        &self,
        mut message: Message,
        caller: Arc<dyn Session>,
        mut control: Control,
        machine: Arc<Machine>,
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

        let binding = match self.listen_bindings.get(&endpoints.local) {
            // MAKE THIS LOCAL AGAIN
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
            endpoints,
        });
        session.receive(message, control, machine)?;
        Ok(())
    }

    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId {
    endpoints: Endpoints,
    recipient: Recipient,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenAndListenError {
    #[error("{0}")]
    Open(#[from] OpenError),
    #[error("{0}")]
    Listen(#[from] ListenError),
}
