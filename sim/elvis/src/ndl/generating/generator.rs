//! Main generator file for ndl
//! Calls the methods needed to completely generate a sim from a parse

use super::{machine_generator, network_generator};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::Internet;

// TODO: Note, the same IP between two different networks seems to break the sim
/// Core Generator calls generating functions to build a sim and then run it
pub async fn core_generator(s: Sim) {
    let mut internet = Internet::new();
    let networks = network_generator(s.networks, &mut internet).unwrap();
    machine_generator(s.machines, &mut internet, networks);
    internet.run().await;
}
