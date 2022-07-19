use super::ipv4_address::Ipv4Address;
use crate::core::control::{from_impls, make_key, ControlValue};
use thiserror::Error as ThisError;

make_key!(LocalAddressKey);
/// A [`ControlValue`] for the local IPv4 address.
pub type LocalAddress = ControlValue<{ LocalAddressKey::KEY }, Ipv4Address>;
from_impls!(LocalAddress, Ipv4Address);
from_impls!(LocalAddress, [u8; 4]);
from_impls!(LocalAddress, u32);

make_key!(RemoteAddressKey);
/// A [`ControlValue`] for the remote IPv4 address.
pub type RemoteAddress = ControlValue<{ RemoteAddressKey::KEY }, Ipv4Address>;
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
