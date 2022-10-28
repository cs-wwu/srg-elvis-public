use elvis::simulations::ping_pong;
use elvis_core::{logging::init_events};
#[tokio::main]
async fn main() {
    init_events();
    ping_pong().await
}
