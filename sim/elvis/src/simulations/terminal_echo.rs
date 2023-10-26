use crate::applications::Terminal;
use elvis_core::{
    new_machine,
    protocols::udp::Udp,
    run_internet, ExitStatus,
};

/// In this simulation, a machine with the Terminal protocol establishes a connection
/// over the local port 8080 and runs until the user closes the connection.
/// Messages sent through the terminal will be echoed back while the connection is available.
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
