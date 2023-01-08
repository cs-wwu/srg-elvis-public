use elvis::{simulations::parse, cli::initialize_from_arguments};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    parse().await
}
