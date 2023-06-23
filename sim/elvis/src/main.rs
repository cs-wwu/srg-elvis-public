//use elvis::cli::initialize_from_arguments;
use elvis::simulations::socket_basic;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
    for _ in 0..1 {
        socket_basic().await;
    }
    println!("Done");
}
