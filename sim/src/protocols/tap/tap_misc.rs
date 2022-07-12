use crate::core::{
    control::{from_impls, ControlValue},
    ProtocolId,
};
use std::error::Error;
use thiserror::Error as ThisError;

pub type NetworkIndex = ControlValue<u8, "tap_network_index">;
from_impls!(NetworkIndex, u8);

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
