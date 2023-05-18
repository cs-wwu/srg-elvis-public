//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocol::{DemuxError, StartError},
    protocols::pci::Pci,
    session::SharedSession,
    Control, FxDashMap, Protocol, Shutdown,
};
use dashmap::mapref::entry::Entry;
use rustc_hash::FxHashMap;
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

pub mod ipv4_parsing;
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_session;
pub use ipv4_session::AddressPair;
use ipv4_session::Ipv4Session;

use super::pci;

pub type DemuxInfo = AddressPair;

/// An implementation of the Internet Protocol.
pub struct Ipv4 {
    listen_bindings: FxDashMap<Ipv4Address, TypeId>,
    sessions: FxDashMap<AddressPair, Arc<Ipv4Session>>,
    recipients: Recipients,
}

impl Ipv4 {
    /// Creates a new instance of the protocol.
    pub fn new(recipients: Recipients) -> Self {
        Self {
            listen_bindings: Default::default(),
            sessions: Default::default(),
            recipients,
        }
    }

    pub fn open(
        &self,
        upstream: TypeId,
        endpoints: AddressPair,
        protocols: ProtocolMap,
    ) -> Result<Arc<Ipv4Session>, OpenError> {
        // TODO(hardint): Possibly make the receiver part of the session ID and just return an
        // existing session as needed
        match self.sessions.entry(endpoints) {
            Entry::Occupied(_) => Err(OpenError::Exists(endpoints)),
            Entry::Vacant(entry) => {
                // If the session does not exist, create it
                let recipient = match self.recipients.get(&endpoints.remote) {
                    Some(recipient) => *recipient,
                    None => {
                        return Err(OpenError::UnknownRecipient(endpoints.remote));
                    }
                };
                let pci_session = protocols.protocol::<Pci>().unwrap().open(recipient.slot);
                let session = Arc::new(Ipv4Session {
                    pci_session,
                    upstream,
                    endpoints,
                    recipient,
                });
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    pub fn listen(&self, upstream: TypeId, address: Ipv4Address) -> Result<(), ListenError> {
        match self.listen_bindings.entry(address) {
            Entry::Occupied(_) => Err(ListenError::Exists(address)),
            Entry::Vacant(entry) => {
                entry.insert(upstream);
                Ok(())
            }
        }
    }
}

// TODO(hardint): Add a static IP lookup table in the constructor so that
// messages can be sent to the correct network

impl Protocol for Ipv4 {
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
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

    #[tracing::instrument(name = "Ipv4::demux", skip_all)]
    fn demux(
        &self,
        mut message: Message,
        _caller: SharedSession,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Extract identifying information from the header and the context and
        // add header information to the context
        let header = match Ipv4Header::from_bytes(message.iter()) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        message.remove_front(header.ihl as usize * 4);
        let endpoints = AddressPair {
            local: header.destination,
            remote: header.source,
        };
        control.insert(endpoints);
        let session = match self.sessions.entry(endpoints) {
            Entry::Occupied(entry) => entry.get().clone(),

            Entry::Vacant(entry) => {
                // If the session does not exist, see if we have a listen
                // binding for it
                let upstream = match self.listen_bindings.get(&endpoints.local) {
                    Some(binding) => *binding,
                    None => {
                        // If we don't have a normal listen binding, check for
                        // a 0.0.0.0 binding
                        match self.listen_bindings.get(&Ipv4Address::CURRENT_NETWORK) {
                            Some(binding) => *binding,
                            None => {
                                tracing::error!(
                                    "Could not find a listen binding for the local address {}",
                                    endpoints.local
                                );
                                Err(DemuxError::MissingSession)?
                            }
                        }
                    }
                };

                let pci_demux_info = control
                    .get::<pci::DemuxInfo>()
                    .ok_or(DemuxError::MissingContext)?;
                let recipient = Recipient::with_mac(pci_demux_info.slot, pci_demux_info.source);
                let session = Arc::new(Ipv4Session {
                    upstream,
                    pci_session: protocols
                        .protocol::<Pci>()
                        .unwrap()
                        .open(pci_demux_info.slot),
                    endpoints: AddressPair {
                        local: header.destination,
                        remote: header.source,
                    },
                    recipient,
                });
                entry.insert(session.clone());
                session
            }
        };
        session.receive(message, control, protocols)?;
        Ok(())
    }
}

pub type Recipients = FxHashMap<Ipv4Address, Recipient>;

// TODO(hardint): Rename to something like pci::SendInfo and move to the PCI module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipient {
    pub slot: PciSlot,
    pub mac: Option<Mac>,
}

impl Recipient {
    pub fn new(slot: PciSlot, mac: Option<Mac>) -> Self {
        Self { slot, mac }
    }

    pub fn with_mac(slot: PciSlot, mac: Mac) -> Self {
        Self::new(slot, Some(mac))
    }

    pub fn broadcast(slot: PciSlot) -> Self {
        Self::new(slot, None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum OpenError {
    #[error("There is already a session for {0:?}")]
    Exists(AddressPair),
    #[error("The IP table is missing an entry for {0}")]
    UnknownRecipient(Ipv4Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListenError {
    #[error("There is already a session for {0:?}")]
    Exists(Ipv4Address),
}
