use crate::core::{
    control::{
        self,
        value::{from_impls, make_key},
    },
    machine::MachineId,
    protocol::ProtocolId,
    Message,
};
use std::error::Error;
use thiserror::Error as ThisError;

const NETWORK_INDEX_KEY: u64 = make_key("Tap Network Index");
/// A [`control::Value`] for which network to send on or which a message was
/// received from.
pub type NetworkId = control::Value<NETWORK_INDEX_KEY, u32>;
from_impls!(NetworkId, u32);

const FIRST_RESPONDER_KEY: u64 = make_key("First responder");
/// A [`control::Value`] for which network to send on or which a message was
/// received from.
pub type FirstResponder = control::Value<FIRST_RESPONDER_KEY, u64>;
from_impls!(FirstResponder, u64);
from_impls!(FirstResponder, ProtocolId);

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}

#[derive(Debug, Clone)]
pub struct Delivery {
    pub message: Message,
    pub network: NetworkId,
    pub sender: MachineId,
}
