use elvis::simulations::basic;
use elvis_core::cli::parse_cli;
#[tokio::main]
async fn main() {
    parse_cli();
    basic().await
}
