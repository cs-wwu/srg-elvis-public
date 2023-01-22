//! Main generator file for ndl
//! Calls the methods needed to completely generate a sim from a parse

use std::sync::Arc;

use elvis_core::run_internet;

use super::{machine_generator, network_generator};
use crate::ndl::parsing::parsing_data::*;

// TODO: Note, the same IP between two different networks seems to break the sim
/// Core Generator calls generating functions to build a sim and then run it
pub async fn core_generator(s: Sim) {
    let networks = network_generator(s.networks);
    let machines = machine_generator(s.machines, &networks);
    run_internet(
        machines,
        networks
            .nets
            .clone()
            .into_values()
            .collect::<Vec<Arc<elvis_core::Network>>>(),
    )
    .await;
    println!("finished internet?");
}
