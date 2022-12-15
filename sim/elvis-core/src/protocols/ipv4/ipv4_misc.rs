use super::ipv4_address::Ipv4Address;
use crate::control::{
    self,
    value::{from_impls, make_key},
    Key,
};

const LOCAL_ADDRESS_KEY: Key = make_key("IPv4 Local Address");
/// A [`control::Value`] for the local IPv4 address.
pub type LocalAddress = control::Value<LOCAL_ADDRESS_KEY, Ipv4Address>;
from_impls!(LocalAddress, Ipv4Address);
from_impls!(LocalAddress, [u8; 4]);
from_impls!(LocalAddress, u32);

const REMOTE_ADDRESS_KEY: Key = make_key("IPv4 Remote Address");
/// A [`control::Value`] for the remote IPv4 address.
pub type RemoteAddress = control::Value<REMOTE_ADDRESS_KEY, Ipv4Address>;
from_impls!(RemoteAddress, Ipv4Address);
from_impls!(RemoteAddress, [u8; 4]);
from_impls!(RemoteAddress, u32);
