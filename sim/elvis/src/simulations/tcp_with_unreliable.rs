use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    message::Message,
    network::{Latency, NetworkBuilder},
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, Tcp,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Transport,
};
use std::time::Duration;

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_with_unreliable() {
    let network = NetworkBuilder::new()
        .mtu(500)
        .loss_rate(0.5)
        .latency(Latency::variable(
            Duration::ZERO,
            Duration::from_millis(200),
        ))
        .build();
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };

    let sm_addr = Ipv4Address::new([6, 0, 0, 0]);

    let ip_table: IpTable<Recipient> = [(sm_addr, Recipient::with_mac(0, 1))].into_iter().collect();

    let message: Vec<_> = (0..8000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint)
                .transport(Transport::Tcp)
                .local_ip(sm_addr),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            WaitForMessage::new(endpoint, message).transport(Transport::Tcp)
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn tcp_with_unreliable() {
        for _ in 0..5 {
            super::tcp_with_unreliable().await;
        }
    }
}
