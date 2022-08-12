use crate::core::control::{
    self,
    value::{from_impls, make_key},
};
use thiserror::Error as ThisError;

const LOCAL_PORT_KEY: u64 = make_key("UDP Local Port");
/// A [`control::Value`] for the local port number.
pub type LocalPort = control::Value<LOCAL_PORT_KEY, u16>;
from_impls!(LocalPort, u16);

const REMOTE_PORT_KEY: u64 = make_key("UDP Remote Port");
/// A [`control::Value`] for the remote port number.
pub type RemotePort = control::Value<REMOTE_PORT_KEY, u16>;
from_impls!(RemotePort, u16);

#[derive(Debug, ThisError)]
pub(super) enum UdpError {
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
    #[error("Too few bytes to constitute a UDP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    InvalidChecksum { actual: u16, expected: u16 },
    #[error("The number of message bytes differs from the header")]
    LengthMismatch,
    #[error("The UDP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}
