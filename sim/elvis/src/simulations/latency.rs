use crate::applications::{Capture, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    network::{Latency, NetworkBuilder},
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message,
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
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(ip_table.clone()))
                .with(Pci::new([network.clone()]))
                .with(
                    SendMessage::new(vec![Message::new("Hello!")], capture_ip_address, 0xbeef)
                        .process(),
                )
                .build(),
        ),
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(ip_table))
                .with(Pci::new([network.clone()]))
                .with(Capture::new(capture_ip_address, 0xbeef, 1).process())
                .build(),
        ),
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
