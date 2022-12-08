use elvis::{cli::initialize_from_arguments, simulations::telephone_single};

#[tokio::main]
async fn main() {
    initialize_from_arguments();
    telephone_single().await
}
