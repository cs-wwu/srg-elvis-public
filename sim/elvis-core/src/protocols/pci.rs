//! The base-level protocol that communicates directly with networks.

use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::TapSlot,
    message::Message,
    network::SharedTap,
    protocol::{
        Context, DemuxError, ListenError, OpenError, QueryError, SharedProtocol, StartError,
    },
    session::SharedSession,
    Control, Protocol,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

mod pci_session;
use pci_session::PciSession;

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
    pub const SLOT_COUNT_QUERY_KEY: Key = (Self::ID, 0);

    /// Creates a new network tap.
    pub fn new(taps: impl IntoIterator<Item = SharedTap>) -> Self {
        Self {
            sessions: taps
                .into_iter()
                .enumerate()
                .map(|(i, tap)| Arc::new(PciSession::new(tap, i as u32)))
                .collect(),
        }
    }

    /// Creates a new network tap.
    pub fn new_shared(taps: impl IntoIterator<Item = SharedTap>) -> SharedProtocol {
        Arc::new(Self::new(taps))
    }

    pub fn set_tap_slot(slot: TapSlot, control: &mut Control) {
        control.insert((Self::ID, 0), slot);
    }

    pub fn get_tap_slot(control: &Control) -> Result<TapSlot, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u32()?)
    }

    pub fn set_first_responder(id: Id, control: &mut Control) {
        control.insert((Self::ID, 1), id.into_inner());
    }

    pub fn get_first_responder(control: &Control) -> Result<Id, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u64()?.into())
    }
}

impl Protocol for Pci {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        _upstream: Id,
        participants: Control,
        _context: Context,
    ) -> Result<SharedSession, OpenError> {
        let network_id = Pci::get_tap_slot(&participants).or_else(|_| {
            tracing::error!("Missing network ID on context");
            Err(OpenError::MissingContext)
        })?;
        let session = self
            .sessions
            .get(network_id as usize)
            .ok_or_else(|| {
                tracing::error!("Network ID is out of bounds");
                OpenError::Other
            })?
            .clone();
        Ok(session)
    }

    fn listen(
        self: Arc<Self>,
        _upstream: Id,
        _participants: Control,
        _context: Context,
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
        self: Arc<Self>,
        context: Context,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), StartError> {
        for session in self.sessions.iter() {
            session.clone().start(context.protocols.clone());
        }
        tokio::spawn(async move {
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
