use crate::{control::ControlError, protocol::ProtocolId, Control};

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;
pub type Mac = u64;

const NETWORKS_ID: ProtocolId = ProtocolId::from_string("Networks");

pub fn set_destination_mac(mac: Mac, control: &mut Control) {
    control.insert((NETWORKS_ID, 0), mac);
}

pub fn get_destination_mac(control: &Control) -> Result<Mac, ControlError> {
    Ok(control.get((NETWORKS_ID, 0))?.ok_u64()?)
}

mod broadcast;
pub use broadcast::Broadcast;

mod direct;
pub use direct::Direct;
