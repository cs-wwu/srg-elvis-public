use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    message::Message,
    network::NetworkBuilder,
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        Endpoint, Pci, Udp,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn udp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let endpoint = Endpoint::new([123, 45, 67, 89].into(), 0xbeef);
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..1_000_000_000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let mut messages = vec![];
    let mut remainder = message.clone();
    while remainder.len() > 1450 {
        let part = remainder.cut(1450);
        messages.push(part);
    }
    messages.push(remainder);
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
                    WaitForMessage::new(endpoint, message)
                        .disable_checking()
                        .process(),
                )
                .build(),
        ),
    ];

    run_internet(&machines).await;
}
