use crate::applications::{SendMessage, ThroughputTester};
use elvis_core::{
    network::{Baud, NetworkBuilder, Throughput},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message,
};
use std::time::Duration;

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn throughput() {
    const UDP_HEADER_SIZE: u64 = 8;
    const IP_HEADER_SIZE: u64 = 20;
    const PAYLOAD_LENGTH: u64 = 6;
    const MESSAGE_LENGTH: u64 = UDP_HEADER_SIZE + IP_HEADER_SIZE + PAYLOAD_LENGTH;
    let network = NetworkBuilder::new()
        .throughput(Throughput::constant(Baud::bytes_per_second(MESSAGE_LENGTH)))
        .build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new("Hello!");
    let messages: Vec<_> = (0..3).map(|_| message.clone()).collect();
    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            SendMessage::new(messages, capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.clone()]).shared(),
            ThroughputTester::new(
                capture_ip_address,
                0xbeef,
                3,
                Duration::from_millis(900)..Duration::from_millis(1100),
            )
            .shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn throughput() {
        super::throughput().await
    }
}
