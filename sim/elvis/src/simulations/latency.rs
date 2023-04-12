use std::time::{Duration, SystemTime};

use crate::applications::{Capture, SendMessage};
use elvis_core::{
    network::{Latency, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn latency() {
    let network = NetworkBuilder::new()
        .latency(Latency::constant(Duration::from_secs(1)))
        .build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let capture = Capture::new(capture_ip_address, 0xbeef, 1).shared();
    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            SendMessage::new(vec![Message::new("Hello!")], capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.clone()]).shared(),
            capture.clone(),
        ]),
    ];

    let now = SystemTime::now();
    run_internet(machines, vec![network]).await;
    assert!(now.elapsed().unwrap().as_millis() >= 1000);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn latency() {
        super::latency().await
    }
}
