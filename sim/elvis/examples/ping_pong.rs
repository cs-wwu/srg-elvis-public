use elvis::{cli::initialize_from_arguments, simulations::ping_pong};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    ping_pong().await
}
