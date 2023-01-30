use crate::applications::{Capture, SendMessage};
use elvis_core::{
    network::{Latency, NetworkBuilder},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
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
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    let capture = Capture::new(capture_ip_address, 0xbeef).shared();
    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            Pci::new_shared([network.tap()]),
            SendMessage::new(Message::new("Hello!"), capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network.tap()]),
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
