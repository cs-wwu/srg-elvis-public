//! Main generator file for ndl
//! Calls the methods needed to completely generate a sim from a parse

use super::{machine_generator, network_generator};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::{run_internet,run_internet_with_timeout};
use std::time::Duration;

/// Core Generator calls generating functions to build a sim and then run it
pub async fn core_generator(s: Sim) {
    let networks = network_generator(s.networks);
    let machines = machine_generator(s.machines, &networks);
    // run_internet_with_timeout(&machines, Duration::from_secs(10)).await;
    run_internet(&machines).await;
}
