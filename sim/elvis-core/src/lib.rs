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
//! - [`Message`](message::Message) and [`Control`] provide basic utilities
//!   common to most protocols
//! - [`Protocol`] and [`Session`] implement individual protocols
//! - [`Internet`] provides the actual simulation
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
pub mod protocols;

pub mod control;
pub use control::Control;

pub mod message;
pub use message::Message;

pub mod protocol;
pub use protocol::Protocol;

pub mod session;
pub use session::Session;

pub mod network;
pub use network::Network;

mod machine;
pub use machine::Machine;

mod internet;
pub use internet::run_internet;

mod id;
pub use id::Id;
