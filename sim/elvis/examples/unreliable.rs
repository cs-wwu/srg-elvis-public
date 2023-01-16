use elvis::{cli::initialize_from_arguments, simulations::unreliable};

#[tokio::main]
async fn main() {
    initialize_from_arguments().await;
    unreliable().await
}
