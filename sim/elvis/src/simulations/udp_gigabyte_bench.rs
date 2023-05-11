use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    message::Message,
    network::NetworkBuilder,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        Pci, Udp,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn udp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
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
                .udp(Udp::new())
                .ipv4(Ipv4::new(ip_table.clone()))
                .pci(Pci::new([network.clone()]))
                .other(SendMessage::new(messages, capture_ip_address, 0xbeef).shared())
                .build(),
        ),
        Machine::new(
            ProtocolMapBuilder::new()
                .udp(Udp::new())
                .ipv4(Ipv4::new(ip_table))
                .pci(Pci::new([network.clone()]))
                .other(
                    WaitForMessage::new(capture_ip_address, 0xbeef, message)
                        .disable_checking()
                        .shared(),
                )
                .build(),
        ),
    ];

    run_internet(machines, vec![network]).await;
}
