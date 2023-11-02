use crate::applications::Terminal;
use elvis_core::{
    new_machine,
    protocols::udp::Udp,
    run_internet, ExitStatus,
};

/// In this simulation, two machines with the Terminal protocol establish connections
/// over the local ports 8080 and a randomly chosen available port and runs until either user closes their connection.
/// Messages sent through the terminals will be echoed back while the connections are available.
pub async fn terminal_echo_double() {
    println!("Begin test");

    let machines = vec![
        new_machine![
            Udp::new(),
            Terminal::new(String::from("localhost:8080")),
        ],
        new_machine![
            Udp::new(),
            Terminal::new(String::from("localhost:0")),
        ],
    ];

    let status = run_internet(&machines).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn terminal_echo_double() {
        super::terminal_echo_double().await;
    }
}
