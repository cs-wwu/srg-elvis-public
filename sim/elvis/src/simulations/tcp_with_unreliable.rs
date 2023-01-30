use crate::applications::{Capture, SendMessage, Transport};
use elvis_core::{
    message::Message,
    network::{Latency, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
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
        .latency(Latency::variable(Duration::ZERO, Duration::from_secs(2)))
        .loss_rate(0.5)
        .build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    let message: Vec<_> = (0..3000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let capture = Capture::new(capture_ip_address, 0xbeef)
        .transport(Transport::Tcp)
        .shared();
    let machines = vec![
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SendMessage::new(message.clone(), capture_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()]).shared(),
            capture.clone(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_with_unreliable() {
        super::tcp_with_unreliable().await
    }
}
