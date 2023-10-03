use elvis::{cli::initialize_from_arguments, simulations::socket_basic};
use elvis_core::protocols::socket_api::socket::SocketType;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
    socket_basic(SocketType::Stream, 3).await;
    socket_basic(SocketType::Datagram, 2).await;
    println!("Done");
}
