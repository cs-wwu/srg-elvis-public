use crate::{parsing::generate_sim};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn parse() {
    let file_path = "./elvis/src/parsing/testing.txt";
    generate_sim(file_path);
}
