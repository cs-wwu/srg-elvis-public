//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocol::{Context, DemuxError, ListenError, OpenError, QueryError, StartError, NotifyError},
    protocols::pci::Pci,
    session::SharedSession,
    Control, FxDashMap, Network, Protocol, Shutdown,
};
use dashmap::mapref::entry::Entry;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::Barrier;

pub mod ipv4_parsing;
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

mod ipv4_session;
use ipv4_session::{Ipv4Session, SessionId};

/// An implementation of the Internet Protocol.
pub struct Ipv4 {
    listen_bindings: FxDashMap<Ipv4Address, Id>,
    sessions: FxDashMap<SessionId, Arc<Ipv4Session>>,
    recipients: Recipients,
}

impl Ipv4 {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(4);

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

    pub fn set_local_address(address: Ipv4Address, control: &mut Control) {
        control.insert((Self::ID, 0), address.to_u32());
    }

    pub fn get_local_address(control: &Control) -> Result<Ipv4Address, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u32()?.into())
    }

    pub fn set_remote_address(address: Ipv4Address, control: &mut Control) {
        control.insert((Self::ID, 1), address.to_u32());
    }

    pub fn get_remote_address(control: &Control) -> Result<Ipv4Address, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u32()?.into())
    }
}

// TODO(hardint): Add a static IP lookup table in the constructor so that
// messages can be sent to the correct network

impl Protocol for Ipv4 {
    fn id(&self) -> Id {
        Self::ID
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
        upstream: Id,
        mut participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let key = SessionId::new(
            Self::get_local_address(&participants).map_err(|_| {
                tracing::error!("Missing local address on context");
                OpenError::MissingContext
            })?,
            Self::get_remote_address(&participants).map_err(|_| {
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
                Pci::set_pci_slot(recipient.slot, &mut participants);
                let tap_session = protocols
                    .protocol(Pci::ID)
                    .expect("No such protocol")
                    .open(Self::ID, participants, protocols)?;
                let session = Arc::new(Ipv4Session::new(tap_session, upstream, key, recipient));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    #[tracing::instrument(name = "Ipv4::listen", skip_all)]
    fn listen(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        let local = Self::get_local_address(&participants).map_err(|_| {
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
            .protocol(Pci::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)
    }

    #[tracing::instrument(name = "Ipv4::demux", skip_all)]
    fn demux(
        &self,
        mut message: Message,
        caller: SharedSession,
        mut context: Context,
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

        Self::set_local_address(identifier.local, &mut context.control);
        Self::set_remote_address(identifier.remote, &mut context.control);

        let session = match self.sessions.entry(identifier) {
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
                let slot = Pci::get_pci_slot(&context.control).map_err(|_| {
                    tracing::error!("Missing network ID on context");
                    DemuxError::MissingContext
                })?;
                let mac = Network::get_sender(&context.control).map_err(|_| {
                    tracing::error!("Missing sender MAC on context");
                    DemuxError::MissingContext
                })?;
                let destination = Recipient::with_mac(slot, mac);
                let session = Arc::new(Ipv4Session::new(caller, *binding, identifier, destination));
                entry.insert(session.clone());
                session
            }
        };
        session.receive(message, context)?;
        Ok(())
    }

    fn query(&self, _key: Key) -> Result<Primitive, QueryError> {
        Err(QueryError::NonexistentKey)
    }

    fn notify(&self, _context: Context) -> Result<(), NotifyError> {
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
