use elvis::{cli::initialize_from_arguments, simulations::socket_basic};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    socket_basic().await
}
