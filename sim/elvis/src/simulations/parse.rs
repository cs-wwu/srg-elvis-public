use crate::ndl::generate_sim;

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn parse() {
    let file_path: String = "./elvis/src/ndl/testing.txt".to_string();
    generate_sim(file_path).await;
}
