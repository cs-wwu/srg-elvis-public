//use elvis::cli::initialize_from_arguments;
//use elvis::simulations::ping_pong_multi;
//use elvis::simulations::socket_basic;
//use elvis::simulations::socket_ping_pong;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
    socket_basic().await;
    println!("Done");
}
