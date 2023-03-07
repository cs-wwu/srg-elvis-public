use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::{Latency, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        pci::PciMonitors,
        tcp::TcpMonitors,
        Pci, Tcp,
    },
    run_internet, Machine,
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
    let dst_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(dst_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();
    let pci_monitors = PciMonitors::new();
    let tcp_monitors = TcpMonitors::new();

    let message: Vec<_> = (0..8000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            SendMessage::new(vec![message.clone()], dst_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            WaitForMessage::new(dst_ip_address, 0xbeef, message)
                .transport(Transport::Tcp)
                .shared(),
        ]),
    ];

    run_internet(
        machines,
        vec![network],
        pci_monitors
            .into_iter()
            .chain(tcp_monitors.into_iter())
            .collect(),
    )
    .await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_with_unreliable() {
        super::tcp_with_unreliable().await
    }
}
