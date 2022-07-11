use crate::core::{Control, NetworkLayerError, ProtocolId};
use std::error::Error;
use thiserror::Error as ThisError;

pub type NetworkIndex = u8;

static NETWORK_INDEX_KEY: &str = "tap_network_index";

pub fn set_network_index(control: &mut Control, index: NetworkIndex) {
    control.insert(NETWORK_INDEX_KEY, index)
}

pub fn get_network_index(control: &Control) -> NetworkIndex {
    control
        .get(NETWORK_INDEX_KEY)
        .expect("Missing network index")
        .to_u8()
        .expect("Incorrect network index type")
}

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("The header did not represent a valid protocol ID: {0}")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
