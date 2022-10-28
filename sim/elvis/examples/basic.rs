use elvis::simulations::basic;
// use elvis_core::{logging::init_events};
use tracing_subscriber;
#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::default()).unwrap();
    basic().await
}
