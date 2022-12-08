use elvis::simulations::latent;
use elvis_core::cli::parse_cli;

#[tokio::main]
async fn main() {
    parse_cli();
    latent().await
}
