use super::ipv4_address::Ipv4Address;
use crate::core::control::{from_impls, ControlValue};
use thiserror::Error as ThisError;

pub type LocalAddress = ControlValue<Ipv4Address, "ipv4_local_address">;
from_impls!(LocalAddress, Ipv4Address);
from_impls!(LocalAddress, [u8; 4]);
from_impls!(LocalAddress, u32);

pub type RemoteAddress = ControlValue<Ipv4Address, "ipv4_remote_address">;
from_impls!(RemoteAddress, Ipv4Address);
from_impls!(RemoteAddress, [u8; 4]);
from_impls!(RemoteAddress, u32);

#[derive(Debug, ThisError)]
pub(super) enum Ipv4Error {
    #[error("Could not find a listen binding for the local address: {0}")]
    MissingListenBinding(LocalAddress),
    #[error("Attempting to create a binding that already exists for local address {0}")]
    BindingExists(LocalAddress),
    #[error("Attempting to create a session that already exists for {0} -> {1}")]
    SessionExists(LocalAddress, RemoteAddress),
}
