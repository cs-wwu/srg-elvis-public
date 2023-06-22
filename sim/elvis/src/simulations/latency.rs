use crate::applications::{Capture, SendMessage};
use elvis_core::{
    network::{Latency, NetworkBuilder},
    new_machine,
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet, Message,
};
use std::time::{Duration, SystemTime};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn latency() {
    let network = NetworkBuilder::new()
        .latency(Latency::constant(Duration::from_secs(1)))
        .build();
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![Message::new("Hello!")], endpoint).process()
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1).process()
        ],
    ];

    let now = SystemTime::now();
    run_internet(&machines).await;
    assert!(now.elapsed().unwrap().as_millis() >= 1000);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn latency() {
        super::latency().await
    }
}
