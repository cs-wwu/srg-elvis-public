use elvis::{cli::initialize_from_arguments, simulations::telephone_multi};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    telephone_multi().await
}
