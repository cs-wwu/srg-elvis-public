use super::ipv4_address::Ipv4Address;
use crate::core::Control;
use thiserror::Error as ThisError;

pub static LOCAL_ADDRESS_KEY: &str = "ipv4_local_address";
pub static REMOTE_ADDRESS_KEY: &str = "ipv4_remote_address";

pub fn get_local_address(control: &Control) -> Ipv4Address {
    control
        .get(LOCAL_ADDRESS_KEY)
        .expect("Missing local address")
        .to_u32()
        .expect("Incorrect local address type")
        .into()
}

pub fn get_remote_address(control: &Control) -> Ipv4Address {
    control
        .get(REMOTE_ADDRESS_KEY)
        .expect("Missing remote address")
        .to_u32()
        .expect("Incorrect remote address type")
        .into()
}

#[derive(Debug, ThisError)]
pub enum Ipv4Error {
    #[error("Could not find a listen binding for the local address: {0}")]
    MissingListenBinding(Ipv4Address),
    #[error("Attempting to create a binding that already exists for source address {0}")]
    BindingExists(Ipv4Address),
    #[error("Attempting to create a session that already exists for {0} -> {1}")]
    SessionExists(Ipv4Address, Ipv4Address),
    #[error("Could not find a session for the key {0} -> {1}")]
    MissingSession(Ipv4Address, Ipv4Address),
}
