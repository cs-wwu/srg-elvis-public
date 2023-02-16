use elvis::simulations;
use std::env;

/// Without arguments, main runs the default simulation
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    println!("Running default simulation...");
    simulations::router_multi().await;
    println!("Done");
}
