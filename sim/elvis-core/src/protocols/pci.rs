//! The base-level protocol that communicates directly with networks.

use crate::{
    control::{ControlError, Key, Primitive},
    gcd::Delivery,
    id::Id,
    internet::NetworkHandle,
    machine::PciSlot,
    message::Message,
    network::{Mac, Mtu},
    protocol::{DemuxError, ListenError, OpenError, QueryError, StartError},
    session::SharedSession,
    Control, Protocol,
};
use std::sync::{Arc, RwLock};

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
#[derive(Default)]
pub struct Pci {
    sessions: RwLock<Vec<Arc<PciSession>>>,
}

impl Pci {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::from_string("PCI");

    /// THe key used the query the number of attached [`Tap`]s
    pub const SLOT_COUNT_QUERY_KEY: Key = (Self::ID, 0);

    /// The key used to query the MTU of the network
    pub const MTU_QUERY_KEY: Key = (Self::ID, 1);

    /// Creates a new network tap.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new network tap.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn receive(&self, delivery: Delivery) {
        match self
            .sessions
            .read()
            .unwrap()
            .iter()
            .find(|session| session.network == delivery.network)
            .expect("This PCI is not connected to the given network")
            .receive(delivery)
        {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e),
        }
    }

    pub fn connect(&self, network_handle: NetworkHandle, mac: Mac, mtu: Mtu) {
        let mut lock = self.sessions.write().unwrap();
        let slot = lock.len();
        lock.push(Arc::new(PciSession::new(
            network_handle,
            mac,
            mtu,
            slot.try_into().unwrap(),
        )));
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

    fn open(&self, _upstream: Id, participants: Control) -> Result<SharedSession, OpenError> {
        let pci_slot = Pci::get_pci_slot(&participants).map_err(|_| {
            eprintln!("Missing PCI slot on context");
            OpenError::MissingContext
        })?;
        let session = self
            .sessions
            .read()
            .unwrap()
            .get(pci_slot as usize)
            .ok_or_else(|| {
                eprintln!("PCI slot is out of bounds");
                OpenError::Other
            })?
            .clone();
        Ok(session)
    }

    fn listen(&self, _upstream: Id, _participants: Control) -> Result<(), ListenError> {
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: SharedSession,
        _control: Control,
    ) -> Result<(), DemuxError> {
        panic!("Cannot demux on a Pci")
    }

    fn start(&self) -> Result<(), StartError> {
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Self::SLOT_COUNT_QUERY_KEY => Ok((self.sessions.read().unwrap().len() as u64).into()),
            _ => Err(QueryError::NonexistentKey),
        }
    }
}
