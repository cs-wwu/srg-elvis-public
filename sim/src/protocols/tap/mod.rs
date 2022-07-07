use std::error::Error;

use thiserror::Error as ThisError;

use crate::core::{NetworkLayerError, ProtocolContextError};

mod protocol;
pub use protocol::Tap;

mod session;
pub use session::TapSession;

type NetworkIndex = u8;

/// The key for a network index on [`elvis::core::Control`]. Expects a value
/// of type `u8`.
pub const NETWORK_INDEX_KEY: &str = "tap_network_index";

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("The header did not represent a valid protocol ID: {0}")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Could not find a protocol for the protocol ID: {0}")]
    NoSuchProtocol(#[from] ProtocolContextError),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
