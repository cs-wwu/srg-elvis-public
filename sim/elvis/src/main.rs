use elvis::cli::initialize_from_arguments;
use std::env;

/// Without arguments, will do nothing
#[tokio::main]
pub async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    initialize_from_arguments().await;
    println!("Done");
}
