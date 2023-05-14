use crate::applications::{Capture, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    message::Message,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci, UserProcess,
    },
    run_internet, Machine, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
    let network = Network::basic();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new("Hello!");
    let machines = vec![
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(ip_table.clone()))
                .with(Pci::new([network.clone()]))
                .with(SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef).process())
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

    run_internet(&machines).await;
    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
        .into_inner()
        .protocol::<UserProcess<Capture>>()
        .unwrap()
        .application()
        .message();
    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn basic() {
        super::basic().await
    }
}
