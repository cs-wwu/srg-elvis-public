use elvis::simulations::telephone_multi;
use elvis_core::{logging::init_events};
#[tokio::main]
async fn main() {
    init_events();
    telephone_multi().await
}
