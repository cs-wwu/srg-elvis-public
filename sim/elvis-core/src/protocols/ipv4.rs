//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocol::{DemuxError, ListenError, OpenError, StartError},
    protocols::pci::Pci,
    session::SharedSession,
    Control, FxDashMap, Participants, Protocol, Shutdown,
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
use ipv4_session::{Ipv4Session, SessionId};

/// An implementation of the Internet Protocol.
pub struct Ipv4 {
    listen_bindings: FxDashMap<Ipv4Address, TypeId>,
    sessions: FxDashMap<SessionId, Arc<Ipv4Session>>,
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

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
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

    #[tracing::instrument(name = "Ipv4::open", skip_all)]
    fn open(
        &self,
        upstream: TypeId,
        mut participants: Participants,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let key = SessionId::new(
            participants.local.address.ok_or_else(|| {
                tracing::error!("Missing local address on context");
                OpenError::MissingContext
            })?,
            participants.remote.address.ok_or_else(|| {
                tracing::error!("Missing remote address on context");
                OpenError::MissingContext
            })?,
        );

        match self.sessions.entry(key) {
            Entry::Occupied(_) => {
                tracing::error!(
                    "A session already exists for {} -> {}",
                    key.local,
                    key.remote
                );
                Err(OpenError::Existing)
            }

            Entry::Vacant(entry) => {
                // If the session does not exist, create it
                let recipient = match self.recipients.get(&key.remote) {
                    Some(tap_slot) => *tap_slot,
                    None => {
                        tracing::error!("No tap slot found for the IP {}", key.remote);
                        return Err(OpenError::Other);
                    }
                };
                participants.slot = Some(recipient.slot);
                let tap_session = protocols
                    .protocol::<Pci>()
                    .expect("No such protocol")
                    .open(TypeId::of::<Self>(), participants, protocols)?;
                let session = Arc::new(
                    Ipv4Session::new(tap_session, upstream, key, recipient).ok_or_else(|| {
                        tracing::error!(
                            "Could not get protocol number for the given upstream protocol"
                        );
                        OpenError::Other
                    })?,
                );
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    #[tracing::instrument(name = "Ipv4::listen", skip_all)]
    fn listen(
        &self,
        upstream: TypeId,
        participants: Participants,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        let local = participants.local.address.ok_or_else(|| {
            tracing::error!("Missing local address on context");
            ListenError::MissingContext
        })?;

        match self.listen_bindings.entry(local) {
            Entry::Occupied(_) => {
                tracing::error!("A binding already exists for local address {}", local);
                Err(ListenError::Existing)?
            }

            Entry::Vacant(entry) => {
                entry.insert(upstream);
            }
        }

        // Essentially a no-op but good for completeness and as an example
        protocols
            .protocol::<Pci>()
            .expect("No such protocol")
            .listen(TypeId::of::<Self>(), participants, protocols)
    }

    #[tracing::instrument(name = "Ipv4::demux", skip_all)]
    fn demux(
        &self,
        mut message: Message,
        caller: SharedSession,
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
        let identifier = SessionId::new(header.destination, header.source);

        control.local.address = Some(identifier.local);
        control.remote.address = Some(identifier.remote);

        let session =
            match self.sessions.entry(identifier) {
                Entry::Occupied(entry) => entry.get().clone(),

                Entry::Vacant(entry) => {
                    // If the session does not exist, see if we have a listen
                    // binding for it
                    let binding = match self.listen_bindings.get(&identifier.local) {
                        Some(binding) => binding,
                        None => {
                            // If we don't have a normal listen binding, check for
                            // a 0.0.0.0 binding
                            let any_listen_id = Ipv4Address::CURRENT_NETWORK;
                            match self.listen_bindings.get(&any_listen_id) {
                                Some(any_binding) => any_binding,
                                None => {
                                    tracing::error!(
                                        "Could not find a listen binding for the local address {}",
                                        identifier.local
                                    );
                                    Err(DemuxError::MissingSession)?
                                }
                            }
                        }
                    };
                    let slot = control.slot.ok_or_else(|| {
                        tracing::error!("Missing network ID on context");
                        DemuxError::MissingContext
                    })?;
                    let mac = control.remote.mac.ok_or_else(|| {
                        tracing::error!("Missing sender MAC on context");
                        DemuxError::MissingContext
                    })?;
                    let destination = Recipient::with_mac(slot, mac);
                    let session = Arc::new(
                    Ipv4Session::new(caller, *binding, identifier, destination).ok_or_else(|| {
    tracing::error!("Could not get a protocol number for the given upstream protocol");
    DemuxError::Other
                    })?
                    );
                    entry.insert(session.clone());
                    session
                }
            };
        session.receive(message, control, protocols)?;
        Ok(())
    }
}

pub type Recipients = FxHashMap<Ipv4Address, Recipient>;

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
