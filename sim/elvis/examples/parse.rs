use elvis::{cli::initialize_from_arguments, simulations::parse};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    parse().await
}
