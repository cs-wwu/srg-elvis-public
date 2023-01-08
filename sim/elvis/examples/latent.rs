use elvis::{cli::initialize_from_arguments, simulations::latent};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    latent().await
}
