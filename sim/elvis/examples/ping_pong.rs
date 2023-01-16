use elvis::{cli::initialize_from_arguments, simulations::ping_pong};

#[tokio::main]
async fn main() {
    initialize_from_arguments().await;
    ping_pong().await
}
