//! Fundamental Internet protocols to be used by most simulations.

pub mod ipv4;
pub use ipv4::Ipv4;

pub(crate) mod tap;
pub use tap::MACHINE_ID_KEY;
pub const TAP_ID: ProtocolId = tap::Tap::ID;

pub mod udp;
pub use udp::Udp;

pub mod user_process;
pub use user_process::UserProcess;

use crate::protocol::ProtocolId;

mod utility;

mod tcp;
