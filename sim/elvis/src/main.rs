use elvis::simulations;
use std::env;

/// Without arguments, main runs the default simulation
#[tokio::main]
async fn main() {
    console_subscriber::init();
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    println!("Running default simulation...");
    simulations::basic().await;
    println!("Done");
}
