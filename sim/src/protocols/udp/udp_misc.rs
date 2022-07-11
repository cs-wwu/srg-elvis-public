use crate::core::control::{from_impls, ControlValue};
use thiserror::Error as ThisError;

pub type LocalPort = ControlValue<u16, "udp_local_port">;
from_impls!(LocalPort, u16);
pub type RemotePort = ControlValue<u16, "udp_remote_port">;
from_impls!(RemotePort, u16);

#[derive(Debug, ThisError)]
pub enum UdpError {
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
}
