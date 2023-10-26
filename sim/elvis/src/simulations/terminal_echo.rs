use crate::applications::Terminal;
use elvis_core::{
    new_machine,
    protocols::udp::Udp,
    run_internet, ExitStatus,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn terminal_echo() {
    println!("Begin test");

    let machines = vec![
        new_machine![
            Udp::new(),
            Terminal::new(String::from("localhost:8080")),
        ],
    ];

    let status = run_internet(&machines).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn terminal_echo() {
        super::terminal_echo().await;
    }
}
