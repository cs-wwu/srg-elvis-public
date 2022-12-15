use crate::{
    control::{
        self,
        value::{from_impls, make_key},
        Key,
    },
    protocol::ProtocolId,
};
use std::error::Error;
use thiserror::Error as ThisError;

const NETWORK_INDEX_KEY: Key = make_key("Tap Network Index");
/// A [`control::Value`] for which network to send on or which a message was
/// received from.
pub type NetworkId = control::Value<NETWORK_INDEX_KEY, u32>;
from_impls!(NetworkId, u32);

const FIRST_RESPONDER_KEY: Key = make_key("First responder");
/// A [`control::Value`] for which network to send on or which a message was
/// received from.
pub type FirstResponder = control::Value<FIRST_RESPONDER_KEY, u64>;
from_impls!(FirstResponder, u64);
from_impls!(FirstResponder, ProtocolId);

/// A key to use with [`Session::query`](crate::Session::query) to get the ID of
/// the machine a session belongs to.
pub const MACHINE_ID_KEY: Key = make_key("First responder");

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
