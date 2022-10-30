use elvis::simulations::ping_pong;
use elvis_core::cli::parse_cli;
#[tokio::main]
async fn main() {
    parse_cli();
    ping_pong().await
}
