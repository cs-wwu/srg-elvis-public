use elvis::simulations::ping_pong;

#[tokio::main]
async fn main() {
    ping_pong().await
}
