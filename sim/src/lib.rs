#![feature(adt_const_params)]
#![feature(core_intrinsics)]
#![feature(const_type_id)]

//! A library for running large-scale simulalations of many computers
//! communicating over networks.
//!
//! Elvis provides a set of primitives to facilitate a variety of
//! networking-related projects:

pub mod applications;
pub mod core;
pub mod protocols;
