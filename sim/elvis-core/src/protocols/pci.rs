//! The base-level protocol that communicates directly with networks.

use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::PciSlot,
    message::Message,
    protocol::{
        Context, DemuxError, ListenError, OpenError, QueryError, SharedProtocol, StartError,
    },
    session::SharedSession,
    Control, Network, Protocol, ProtocolMap, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

mod pci_session;
pub(crate) use pci_session::PciSession;

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
pub struct Pci {
    sessions: Vec<Arc<PciSession>>,
}

impl Pci {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::from_string("PCI");

    /// THe key used the query the number of attached [`Tap`]s
    pub const SLOT_COUNT_QUERY_KEY: Key = (Self::ID, 0);

    /// The key used to query the MTU of the network
    pub const MTU_QUERY_KEY: Key = (Self::ID, 1);

    /// Creates a new network tap.
    pub fn new(networks: impl IntoIterator<Item = Arc<Network>>) -> Self {
        Self {
            sessions: networks
                .into_iter()
                .enumerate()
                .map(|(i, network)| PciSession::new(network, i as u32))
                .collect(),
        }
    }

    /// Creates a new network tap.
    pub fn shared(self) -> SharedProtocol {
        Arc::new(self)
    }

    /// Sets the index of the tap that a message should be sent over or that a
    /// message was received from.
    pub fn set_pci_slot(slot: PciSlot, control: &mut Control) {
        control.insert((Self::ID, 0), slot);
    }

    /// Gets the index of the tap that a message should be sent over or that a
    /// message was received from.
    pub fn get_pci_slot(control: &Control) -> Result<PciSlot, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u32()?)
    }
}

impl Protocol for Pci {
    fn id(&self) -> Id {
        Self::ID
    }

    fn open(
        &self,
        _upstream: Id,
        participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let pci_slot = Pci::get_pci_slot(&participants).map_err(|_| {
            tracing::error!("Missing PCI slot on context");
            OpenError::MissingContext
        })?;
        let session = self
            .sessions
            .get(pci_slot as usize)
            .ok_or_else(|| {
                tracing::error!("PCI slot is out of bounds");
                OpenError::Other
            })?
            .clone();
        Ok(session)
    }

    fn listen(
        &self,
        _upstream: Id,
        _participants: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        Ok(())
    }

    fn demux(
        self: Arc<Self>,
        _message: Message,
        _caller: SharedSession,
        _context: Context,
    ) -> Result<(), DemuxError> {
        panic!("Cannot demux on a Pci")
    }

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        for session in self.sessions.iter() {
            session.clone().start(protocols.clone());
        }
        tokio::spawn(async move {
            // Wait until all the taps have started before starting the sim
            initialized.wait().await;
        });
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Self::SLOT_COUNT_QUERY_KEY => Ok((self.sessions.len() as u64).into()),
            _ => Err(QueryError::NonexistentKey),
        }
    }
}
