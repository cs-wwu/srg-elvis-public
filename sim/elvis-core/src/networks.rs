use crate::{control::ControlError, id::Id, Control};

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;
pub type Mac = u64;

const NETWORKS_ID: Id = Id::from_string("Networks");

pub fn set_destination_mac(mac: Mac, control: &mut Control) {
    control.insert((NETWORKS_ID, 0), mac);
}

pub fn get_destination_mac(control: &Control) -> Result<Mac, ControlError> {
    Ok(control.get((NETWORKS_ID, 0))?.ok_u64()?)
}

mod generic;
pub use generic::Generic;
