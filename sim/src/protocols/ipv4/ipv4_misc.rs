use super::ipv4_address::Ipv4Address;
use crate::core::control::ControlValue;
use thiserror::Error as ThisError;

pub type LocalAddress = ControlValue<Ipv4Address, "ipv4_local_address">;
pub type RemoteAddress = ControlValue<Ipv4Address, "ipv4_remote_address">;

#[derive(Debug, ThisError)]
pub(super) enum Ipv4Error {
    #[error("Could not find a listen binding for the local address: {0}")]
    MissingListenBinding(Ipv4Address),
    #[error("Attempting to create a binding that already exists for source address {0}")]
    BindingExists(Ipv4Address),
    #[error("Attempting to create a session that already exists for {0} -> {1}")]
    SessionExists(Ipv4Address, Ipv4Address),
}
