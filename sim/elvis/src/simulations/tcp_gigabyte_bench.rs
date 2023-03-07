use std::time::Instant;

use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        pci::PciMonitors,
        tcp::TcpMonitors,
        Pci, Tcp,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();
    let pci_monitors = PciMonitors::new();
    let tcp_monitors = TcpMonitors::new();

    let message: Vec<_> = (0..1_000_000_000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            WaitForMessage::new(capture_ip_address, 0xbeef, message)
                .transport(Transport::Tcp)
                .disable_checking()
                .shared(),
        ]),
    ];

    let instant = Instant::now();
    run_internet(
        machines,
        vec![network],
        pci_monitors
            .into_iter()
            .chain(tcp_monitors.into_iter())
            .collect(),
    )
    .await;
    println!("{:?}", instant.elapsed());
}
