use elvis::cli::initialize_from_arguments;
use elvis::simulations::socket_ping_pong;
use elvis::simulations::ping_pong_multi;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
    socket_ping_pong().await;
    println!("Done");
}
