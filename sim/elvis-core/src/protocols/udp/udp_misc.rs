use crate::control::{
    self,
    value::{from_impls, make_key},
    Key,
};

const LOCAL_PORT_KEY: Key = make_key("UDP Local Port");
/// A [`control::Value`] for the local port number.
pub type LocalPort = control::Value<LOCAL_PORT_KEY, u16>;
from_impls!(LocalPort, u16);

const REMOTE_PORT_KEY: Key = make_key("UDP Remote Port");
/// A [`control::Value`] for the remote port number.
pub type RemotePort = control::Value<REMOTE_PORT_KEY, u16>;
from_impls!(RemotePort, u16);
