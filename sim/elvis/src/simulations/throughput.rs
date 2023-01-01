use std::time::Duration;

use crate::applications::{SendMessage, ThroughputTester};
use elvis_core::{
    network::{Baud, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn throughput() {
    const TCP_HEADER_SIZE: u32 = 20;
    const IP_HEADER_SIZE: u32 = 20;
    const PAYLOAD_LENGTH: u32 = 6;
    const MESSAGE_LENGTH: u32 = TCP_HEADER_SIZE + IP_HEADER_SIZE + PAYLOAD_LENGTH;
    let network = NetworkBuilder::new()
        .throughput(Baud::bytes_per_second(MESSAGE_LENGTH))
        .build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    let machines = vec![
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            Pci::new_shared([network.tap()]),
            SendMessage::new_shared("First ", capture_ip_address, 0xbeef, None),
            SendMessage::new_shared("Second", capture_ip_address, 0xbeef, None),
            SendMessage::new_shared("Third ", capture_ip_address, 0xbeef, None),
        ]),
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network.tap()]),
            ThroughputTester::new_shared(
                capture_ip_address,
                0xbeef,
                3,
                Duration::from_millis(900)..Duration::from_millis(1100),
            ),
        ]),
    ];

    run_internet(machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn throughput() {
        super::throughput().await
    }
}
