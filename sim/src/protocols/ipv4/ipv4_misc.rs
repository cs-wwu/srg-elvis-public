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
    #[error("The IPv4 header is incomplete")]
    HeaderTooShort,
    #[error("Could not convert to Reliability from {0}")]
    Reliability(u8),
    #[error("Could not convert to Delay from {0}")]
    Delay(u8),
    #[error("Could not convert to Throughput from {0}")]
    Throughput(u8),
    #[error("Could not convert to Precedence from {0}")]
    Precedence(u8),
    #[error("The reserved bits in type of service are nonzero")]
    UsedReservedTos,
    #[error("Expected version 4 in IPv4 header")]
    IncorrectIpv4Version,
    #[error("The reserved control flags bit was used")]
    UsedReservedFlag,
}
