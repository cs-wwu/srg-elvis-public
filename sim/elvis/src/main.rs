use elvis::cli::parse_args;
use std::env;

/// Without arguments, will do nothing
/// To run a simulation you must provide an Elvis NDL file and with a simulation inside
/// Example run is defined as:
/// cargo run -- --ndl ./elvis/src/ndl/filename.ndl
#[tokio::main]
async fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));
    parse_args().await;
}
