use elvis::cli::parse_args;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    parse_args().await;
}
