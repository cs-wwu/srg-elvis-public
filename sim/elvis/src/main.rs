<<<<<<< HEAD
use elvis::cli::initialize_from_arguments;
use elvis::simulations::socket_ping_pong;
use elvis::simulations::ping_pong_multi;
=======
use elvis::{cli::initialize_from_arguments, simulations::socket_basic};
>>>>>>> 790434ff1e14f709e0346804a0b7a9a9647e5bba
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
<<<<<<< HEAD
    socket_ping_pong().await;
=======
    socket_basic().await;
>>>>>>> 790434ff1e14f709e0346804a0b7a9a9647e5bba
    println!("Done");
}
