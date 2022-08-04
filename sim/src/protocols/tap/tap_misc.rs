use crate::core::{
    control::{from_impls, make_key, ControlValue},
    Delivery, Mtu, Postmarked, ProtocolId,
};
use std::error::Error;
use thiserror::Error as ThisError;
use tokio::sync::mpsc::{Receiver, Sender};

const NETWORK_INDEX_KEY: u64 = make_key("Tap Network Index");
/// A [`ControlValue`] for which network to send on or which a message was
/// received from.
pub type NetworkId = ControlValue<NETWORK_INDEX_KEY, crate::core::NetworkId>;
from_impls!(NetworkId, crate::core::NetworkId);

pub struct NetworkInfo {
    pub mtu: Mtu,
    pub network_id: NetworkId,
    pub sender: Sender<Postmarked>,
    pub receiver: Receiver<Delivery>,
}

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
