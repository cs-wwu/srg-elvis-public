use crate::applications::{SendMessage, ThroughputTester};
use elvis_core::{
    machine::ProtocolMapBuilder,
    network::{Baud, NetworkBuilder, Throughput},
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        udp::Udp,
        Endpoint, Pci,
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
    let endpoint = Endpoint::new([123, 45, 67, 89].into(), 0xbeef);
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new("Hello!");
    let messages: Vec<_> = (0..3).map(|_| message.clone()).collect();
    let machines = vec![
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(ip_table.clone()))
                .with(Pci::new([network.clone()]))
                .with(SendMessage::new(messages, endpoint).process())
                .build(),
        ),
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(ip_table))
                .with(Pci::new([network.clone()]))
                .with(
                    ThroughputTester::new(
                        endpoint,
                        3,
                        Duration::from_millis(900)..Duration::from_millis(1100),
                    )
                    .process(),
                )
                .build(),
        ),
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn throughput() {
        super::throughput().await
    }
}
