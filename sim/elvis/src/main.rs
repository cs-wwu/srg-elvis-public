use elvis::simulations;
use std::time::Instant;

/// Without arguments, main runs the default simulation
#[tokio::main]
async fn main() {
    console_subscriber::init();
    let now = Instant::now();
    simulations::tcp_gigabyte_bench().await;
    println!("{:?}", now.elapsed());
}
