use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        pci::PciMonitors,
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
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();
    let pci_monitors = PciMonitors::new();

    let message: Vec<_> = (0..1_000_000_00).map(|i| i as u8).collect();
    let message = Message::new(message);
    let mut messages = vec![];
    let mut remainder = message.clone();
    while remainder.len() > 1450 {
        let part = remainder.cut(1450);
        messages.push(part);
    }
    messages.push(remainder);
    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            SendMessage::new(messages, capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            WaitForMessage::new(capture_ip_address, 0xbeef, message)
                .disable_checking()
                .shared(),
        ]),
    ];

    run_internet(machines, vec![network], pci_monitors.into_iter().collect()).await;
}
