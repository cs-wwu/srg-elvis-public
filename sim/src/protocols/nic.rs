use crate::core::{
    AwakeContext, ControlFlow, DemuxId, Message, Mtu, NetworkLayer, NetworkLayerError,
    PhysicalAddress, Protocol, ProtocolId, Session,
};
use std::{
    cell::RefCell,
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
    bindings: HashMap<ProtocolId, Weak<RefCell<dyn Protocol>>>,
    sessions: HashMap<ProtocolId, Rc<RefCell<dyn Session>>>,
}

impl Nic {
    /// The unique identifier for this protocol
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    /// Creates a new network interface card.
    ///
    /// # Arguments
    ///
    /// * `mtu` is the minimum transmission unit of the connected network. It is
    ///   the number of bytes in the largest frame the network supports.
    /// * `network_index` is the index of the network this NIC attaches to. When
    ///   using the neighbors iterator on
    ///   [AwakeContext](elvis::core::AwakeContext), the network index refers to
    ///   the nth element of the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use elvis::protocols::Nic;
    /// let _nic = Nic::new(1500, 0);
    /// ```
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
        invoker: Weak<RefCell<dyn Protocol>>,
        _identifier: DemuxId,
    ) -> Result<Weak<RefCell<dyn Session>>, Box<dyn Error>> {
        match self
            .sessions
            .entry((*invoker.upgrade().unwrap()).borrow().id())
        {
            Entry::Occupied(entry) => Ok(Rc::downgrade(entry.get())),
            Entry::Vacant(entry) => {
                let session = Rc::new(RefCell::new(NicSession::new(
                    invoker,
                    self.mtu,
                    self.network_index,
                )));
                let reference = Rc::downgrade(&session);
                entry.insert(session);
                Ok(reference)
            }
        }
    }

    fn open_passive(
        &mut self,
        _invoker: Weak<RefCell<dyn Protocol>>,
        _identifier: DemuxId,
    ) -> Result<Weak<RefCell<dyn Session>>, Box<dyn Error>> {
        Err(Box::new(NicError::PassiveOpen))
    }

    fn add_demux_binding(
        &mut self,
        invoker: Weak<RefCell<dyn Protocol>>,
        _identifier: DemuxId,
    ) -> Result<(), Box<dyn Error>> {
        let id = (*invoker.upgrade().unwrap()).borrow().id();
        match self.bindings.entry(id) {
            Entry::Occupied(_) => Err(Box::new(NicError::BindingExists(id))),
            Entry::Vacant(entry) => {
                entry.insert(invoker.clone());
                Ok(())
            }
        }
    }

    fn demux(&self, message: Message) -> Result<Weak<RefCell<dyn Session>>, Box<dyn Error>> {
        let header = take_header(&message).ok_or(NicError::HeaderLength)?;
        let protocol: ProtocolId = header.try_into()?;
        let session = self
            .sessions
            .get(&protocol)
            .ok_or(NicError::NoSessionForHeader)?;
        Ok(Rc::downgrade(&session))
    }

    fn awake(&mut self, context: &mut AwakeContext) -> Result<ControlFlow, Box<dyn Error>> {
        for session in self.sessions.values_mut() {
            session.borrow_mut().awake(context)?;
        }
        Ok(ControlFlow::Continue)
    }
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}

#[derive(Clone)]
pub struct NicSession {
    // Todo: Provide an API for accessing this value
    mtu: Mtu,
    network_index: usize,
    demuxer: Weak<RefCell<dyn Protocol>>,
}

impl NicSession {
    pub fn new(demuxer: Weak<RefCell<dyn Protocol>>, mtu: Mtu, network_index: usize) -> Self {
        Self {
            demuxer,
            mtu,
            network_index,
        }
    }
}

impl Session for NicSession {
    fn demuxer(&self) -> Weak<RefCell<dyn Protocol>> {
        self.demuxer.clone()
    }

    fn send(&mut self, message: Message, context: &mut AwakeContext) -> Result<(), Box<dyn Error>> {
        context
            .networks()
            .nth(self.network_index)
            .ok_or(Box::new(NicError::NetworkIndex(self.network_index)))?
            .borrow_mut()
            .send(PhysicalAddress::Broadcast, message);
        Ok(())
    }

    fn recv(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        let demuxer = self.demuxer.upgrade().ok_or(NicError::MissingDemuxer)?;
        let message = message.slice(2..);
        demuxer.borrow_mut().demux(message)?;
        Ok(())
    }

    fn awake(&mut self, _context: &mut AwakeContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(ControlFlow::Continue)
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
    NoSessionForHeader,
    #[error("Failed to get a handle to a NIC session demuxer")]
    MissingDemuxer,
    #[error("The network index does not exist: {0}")]
    NetworkIndex(usize),
    #[error("{0}")]
    Other(Box<dyn Error>),
}

#[cfg(test)]
mod tests {
    use super::Nic;
    use crate::core::{DemuxId, Protocol};
    use std::{cell::RefCell, error::Error, rc::Rc};

    #[test]
    fn nic_id() {
        let nic = Nic::new(1500, 0);
        assert_eq!(nic.id(), Nic::ID);
    }

    #[test]
    fn nic_open_active() -> Result<(), Box<dyn Error>> {
        let mut nic1 = Nic::new(1500, 0);
        let nic2: Rc<RefCell<dyn Protocol>> = Rc::new(RefCell::new(Nic::new(1500, 0)));
        nic1.open_active(Rc::downgrade(&nic2), DemuxId::default())?;
        Ok(())
    }

    #[test]
    #[should_panic]
    fn nic_open_passive() {
        let mut nic1 = Nic::new(1500, 0);
        let nic2: Rc<RefCell<dyn Protocol>> = Rc::new(RefCell::new(Nic::new(1500, 0)));
        nic1.open_passive(Rc::downgrade(&nic2), DemuxId::default()).unwrap();
    }
}
