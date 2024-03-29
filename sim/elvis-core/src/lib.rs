//! The Extensible, Large-scale Virtual Internet Simulator, a library for
//! running simulations of many computers communicating over networks.
//!
//! # Uses
//!
//! - Educators can use Elvis as a pedagogical tool. Using simulations, students
//!   can explore how network traffic traverses an internet, run DDOS attacks,
//!   learn how to configure network hardware, and implement networking
//!   protocols, all without the hassle of virtual machines.
//! - Researchers can implement and test novel protocols and technologies in a
//!   sandboxed environment with built-in diagnostics to monitor effects such as
//!   congestion and dropped packets.
//!
//! Foundational abstractions for building Internet simulations.
//!
//! This module contains the necessary pieces to implement protocols and to
//! simulate machines communicating across networks. Elvis follows the
//! [x-kernel] design for protocol layering.
//!
//! # Organization
//! - [`Message`] and [`Control`] provide basic utilities
//!   common to most protocols
//! - [`Protocol`] and [`Session`] implement individual protocols
//! - [`run_internet`] runs the actual simulation
//!
//! # Protocol structure
//!
//! [`Protocol`] and [`Session`] work closely together. A session contains the
//! state for a single open connection on a single protocol. For example, a TCP
//! session would contain information about the window, the state of the
//! connection, and the stream of bytes to send. Sessions are created by the
//! protocol either in response to a program opening a connection or a new
//! connection being opened for a listening server program. In addition to
//! creating new sessions, protocols also route incoming packets to the correct
//! sessions.
//!
//! [x-kernel]: https://ieeexplore.ieee.org/document/67579

mod logging;
pub mod message;
pub mod protocols;

pub mod ip_table;
pub use ip_table::IpTable;

use dashmap::DashMap;
pub use message::Message;

pub mod protocol;
pub use protocol::Protocol;

pub mod session;
pub use session::Session;

pub mod network;
pub use network::Network;

pub mod machine;
pub use machine::Machine;

mod internet;
pub use internet::run_internet;
pub use internet::run_internet_with_timeout;

pub mod shutdown;
pub use shutdown::ExitStatus;
pub use shutdown::Shutdown;

mod transport;
pub use transport::Transport;

mod control;
pub use control::Control;

pub use protocols::arp::subnetting;

use std::hash::BuildHasherDefault;
pub type FxDashMap<K, V> = DashMap<K, V, BuildHasherDefault<rustc_hash::FxHasher>>;
