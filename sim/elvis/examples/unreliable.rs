use elvis::simulations::unreliable;
use elvis_core::cli::parse_cli;

#[tokio::main]
async fn main() {
    parse_cli();
    unreliable().await
}
