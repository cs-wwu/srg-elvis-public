use elvis::simulations::latent;
use elvis_core::{logging::init_events};
#[tokio::main]
async fn main() {
    init_events();
    latent().await
}
