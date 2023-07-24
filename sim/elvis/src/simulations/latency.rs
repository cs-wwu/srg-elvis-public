use crate::applications::{Capture, SendMessage};
use elvis_core::{
    network::{Latency, NetworkBuilder},
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Message,
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

    let local_address: Ipv4Address = [127, 0, 0, 1].into();

    let ip_table: IpTable<Recipient> = [(local_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![Message::new("Hello!")], endpoint)
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1)
        ],
    ];

    let now = SystemTime::now();
    let status = run_internet_with_timeout(&machines, Duration::from_secs(5)).await;

    assert_eq!(status, ExitStatus::Exited);
    assert!(now.elapsed().unwrap().as_millis() >= 1000);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn latency() {
        super::latency().await
    }
}
