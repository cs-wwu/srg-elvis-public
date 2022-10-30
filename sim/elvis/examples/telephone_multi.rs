use elvis::simulations::telephone_multi;
use elvis_core::cli::parse_cli;
#[tokio::main]
async fn main() {
    parse_cli();
    telephone_multi().await
}
