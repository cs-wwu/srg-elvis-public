/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;

mod broadcast;
pub use broadcast::Broadcast;
