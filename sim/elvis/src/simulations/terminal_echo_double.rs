use crate::applications::Terminal;
use elvis_core::{
    new_machine,
    protocols::udp::Udp,
    run_internet, ExitStatus,
};

/// In this simulation, two machines with the Terminal protocol establish
/// connections over the local ports 8080 and a randomly chosen available port
/// and echo messages back until either user closes their connection (^] -> ^D).
/// 
/// To connect to the application, open 2 terminals and call
/// "telnet localhost 8080" in one and "telnet localhost xxxx" in the other,
/// where 'xxxx' is the chosen randomly port printed by the sim.
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
