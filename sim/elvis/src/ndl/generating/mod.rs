//! Mod file for generator: allows use across the Elvis
pub mod generator;
mod generator_data;
mod generator_utils;
mod machine_generator;
mod network_generator;
pub use generator::core_generator;
use machine_generator::machine_generator;
use network_generator::network_generator;
