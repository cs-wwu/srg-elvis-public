use elvis::simulations::telephone_single;
use elvis_core::cli::parse_cli;
#[tokio::main]
async fn main() {
    parse_cli();
    telephone_single().await
}
