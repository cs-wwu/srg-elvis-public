use elvis::simulations::basic;
use elvis_core::logging::init_events;
#[tokio::main]
async fn main() {
    init_events();
    basic().await
}
