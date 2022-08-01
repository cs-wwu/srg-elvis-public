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

pub mod applications;
pub mod core;
pub mod protocols;
pub mod simulation;
