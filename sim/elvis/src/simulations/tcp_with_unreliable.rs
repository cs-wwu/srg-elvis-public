use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::{Latency, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
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

    let message: Vec<_> = (0..8000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            SendMessage::new(vec![message.clone()], dst_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.clone()]).shared(),
            WaitForMessage::new(dst_ip_address, 0xbeef, message)
                .transport(Transport::Tcp)
                .shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_with_unreliable() {
        super::tcp_with_unreliable().await
    }
}
