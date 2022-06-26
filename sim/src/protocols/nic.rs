use crate::core::{Message, Mtu, ParticipantSet, Protocol, ProtocolId, Session, SessionId};
use std::{error::Error, collections::HashMap};
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
    session: Option<SessionId>,
}

impl Nic {
    pub fn new(mtu: Mtu, network_index: usize) -> Self {
        Self {
            mtu,
            network_index,
        }
    }
}

impl Protocol for Nic {
    fn id(&self) -> ProtocolId {
        0
    }

    fn open_active(
        &mut self,
        invoker: ProtocolId,
        participants: ParticipantSet,
    ) -> Option<Box<dyn Session>> {
        todo!()
    }

    fn open_passive(
        &mut self,
        invoker: ProtocolId,
        participants: ParticipantSet,
    ) -> Option<Box<dyn Session>> {
        // There should be no lower protocols that would call this on Nic
        None
    }

    fn add_demux_binding(&mut self, invoker: ProtocolId, participants: ParticipantSet) {
        todo!()
    }

    fn demux(&self, message: Message) -> Result<SessionId, Box<dyn Error>> {
        let header_bytes = take_header(message).ok_or(NicError::HeaderLength)?;
        let protocol = ProtocolId::from_be_bytes(header_bytes);
    }
}

fn take_header(message: Message) -> Option<[u8; 4]> {
    let iter = message.iter();
    Some([iter.next()?, iter.next()?, iter.next()?, iter.next()?])
}

pub struct NicSession {
    mapping: HashMap<ProtocolId, SessionId>,
}

impl Session for NicSession {

}

#[derive(Debug, ThisError)]
pub enum NicError {
    #[error("Expected four bytes for the NIC header")]
    HeaderLength,
}
