use crate::core::control::{from_impls, make_key, ControlValue};
use thiserror::Error as ThisError;

make_key!(LocalPortKey);
pub type LocalPort = ControlValue<{ LocalPortKey::KEY }, u16>;
from_impls!(LocalPort, u16);

make_key!(RemotePortKey);
pub type RemotePort = ControlValue<{ RemotePortKey::KEY }, u16>;
from_impls!(RemotePort, u16);

#[derive(Debug, ThisError)]
pub enum UdpError {
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
}
