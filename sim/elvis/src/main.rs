use elvis::simulations;
use std::time::Instant;

/// Without arguments, main runs the default simulation
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let now = Instant::now();
    simulations::udp_gigabyte_bench().await;
    println!("{:?}", now.elapsed());
}
