use elvis::simulations::socket_basic;
use elvis_core::protocols::socket_api::socket::SocketType;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
pub async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    //initialize_from_arguments().await;
    //socket_basic(SocketType::Datagram, 255, false, 0).await;
    socket_basic(SocketType::Stream, 255, false, 0).await;
    println!("Done");
}
