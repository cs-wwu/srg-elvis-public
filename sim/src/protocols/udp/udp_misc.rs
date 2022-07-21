use crate::core::control::{from_impls, make_key, ControlValue};
use thiserror::Error as ThisError;

make_key!(LocalPortKey);
/// A [`ControlValue`] for the local port number.
pub type LocalPort = ControlValue<{ LocalPortKey::KEY }, u16>;
from_impls!(LocalPort, u16);

make_key!(RemotePortKey);
/// A [`ControlValue`] for the remote port number.
pub type RemotePort = ControlValue<{ RemotePortKey::KEY }, u16>;
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
}
