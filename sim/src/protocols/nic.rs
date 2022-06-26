use crate::core::{
    DemuxId, Message, Mtu, NetworkLayer, NetworkLayerError, Protocol, ProtocolId, Session,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::{Rc, Weak},
};
use thiserror::Error as ThisError;

/// Represents something akin to an Ethernet tap or a network interface card.
/// This should be the first responder to messages coming in off the network. It
/// is simply there to specify which protocol should respond to a raw message
/// coming off the network, for example IPv4 or IPv6. The header is very simple,
/// adding only a u32 that specifies the `ProtocolId` of the protocol that
/// should receive the message.
pub struct Nic {
    network_index: usize,
    mtu: Mtu,
    bindings: HashMap<ProtocolId, Weak<dyn Protocol>>,
    sessions: HashMap<ProtocolId, Rc<dyn Session>>,
}

impl Nic {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    pub fn new(mtu: Mtu, network_index: usize) -> Self {
        Self {
            mtu,
            network_index,
            bindings: Default::default(),
            sessions: Default::default(),
        }
    }
}

impl Protocol for Nic {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        invoker: Rc<dyn Protocol>,
        _identifier: DemuxId,
    ) -> Result<Weak<dyn Session>, Box<dyn Error>> {
        match self.sessions.entry(invoker.id()) {
            Entry::Occupied(entry) => Ok(Rc::downgrade(entry.get())),
            Entry::Vacant(entry) => {
                let session = Rc::new(NicSession::new(Rc::downgrade(&invoker)));
                let reference = Rc::downgrade(&session);
                entry.insert(session);
                Ok(reference)
            }
        }
    }

    fn open_passive(
        &mut self,
        _invoker: Rc<dyn Protocol>,
        _identifier: DemuxId,
    ) -> Result<Weak<dyn Session>, Box<dyn Error>> {
        Err(Box::new(NicError::PassiveOpen))
    }

    fn add_demux_binding(
        &mut self,
        invoker: Rc<dyn Protocol>,
        _identifier: DemuxId,
    ) -> Result<(), Box<dyn Error>> {
        let id = invoker.id();
        match self.bindings.entry(id) {
            Entry::Occupied(_) => Err(Box::new(NicError::BindingExists(id))),
            Entry::Vacant(entry) => {
                entry.insert(Rc::downgrade(&invoker));
                Ok(())
            }
        }
    }

    fn demux(&self, message: Message) -> Result<Weak<dyn Session>, Box<dyn Error>> {
        let header = take_header(&message).ok_or(NicError::HeaderLength)?;
        let protocol: ProtocolId = header.try_into()?;
        let session = self.sessions.get(&protocol).ok_or(NicError::Demux)?;
        Ok(Rc::downgrade(&session))
    }
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}

#[derive(Clone)]
pub struct NicSession {
    demuxer: Weak<dyn Protocol>,
}

impl NicSession {
    pub fn new(demuxer: Weak<dyn Protocol>) -> Self {
        Self { demuxer }
    }
}

impl Session for NicSession {
    fn protocol(&self) -> ProtocolId {
        Nic::ID
    }

    fn demuxer(&self) -> Weak<dyn Protocol> {
        self.demuxer.clone()
    }

    fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn recv(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        // let demuxer = self.demuxer.upgrade().ok_or(NicError::MissingDemuxer)?;
        // let message = message.slice(2, )
        // demuxer.demux(message)
        // Ok(())
        todo!()
    }

    fn awake(&mut self) {
        // No-op
    }
}

#[derive(Debug, ThisError)]
pub enum NicError {
    #[error("Expected two bytes for the NIC header")]
    HeaderLength,
    #[error("The NIC header did not represent a valid protocol ID")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Unexpected passive open on NIC")]
    PassiveOpen,
    #[error("Attempt to create an existing demux binding: {0:?}")]
    BindingExists(ProtocolId),
    #[error("Could not find a matching session for the NIC header")]
    Demux,
    #[error("Failed to get a handle to a NIC session demuxer")]
    MissingDemuxer,
}
