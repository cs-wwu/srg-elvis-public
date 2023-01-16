use elvis::{cli::initialize_from_arguments, simulations::basic};

#[tokio::main]
async fn main() {
    initialize_from_arguments().await;
    basic().await
}
