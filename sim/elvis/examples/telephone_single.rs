use elvis::simulations::telephone_single;
use elvis_core::{logging::init_events};
#[tokio::main]
async fn main() {
    init_events();
    // tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::default()).unwrap();
    telephone_single().await
}
