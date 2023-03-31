use elvis::simulations;
use std::time::Instant;

/// Without arguments, main runs the default simulation
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::default()).unwrap();
    let now = Instant::now();
    simulations::tcp_gigabyte_bench().await;
    println!("{:?}", now.elapsed());
}
