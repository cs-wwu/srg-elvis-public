use elvis::cli::initialize_from_arguments;
use std::env;

/// Without arguments, main runs the default simulation
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    // println!("Running default simulation...");
    initialize_from_arguments().await;
    println!("Done");
}
