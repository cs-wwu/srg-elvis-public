use elvis::simulations::unreliable;
use elvis_core::{logging::init_events};
#[tokio::main]
async fn main() {
    init_events();
    unreliable().await
}
