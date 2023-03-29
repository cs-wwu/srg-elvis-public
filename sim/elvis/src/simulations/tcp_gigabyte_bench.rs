use std::time::Instant;

use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        Pci, Tcp,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_gigabyte_bench() {
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..1_000_000_00).map(|i| i as u8).collect();
    let message = Message::new(message);

    let networks: Vec<_> = (0..10)
        .map(|_| NetworkBuilder::new().mtu(1500).build())
        .collect();

    let machines: Vec<_> = networks
        .iter()
        .flat_map(|network| {
            [
                Machine::new([
                    Tcp::new().shared() as SharedProtocol,
                    Ipv4::new(ip_table.clone()).shared(),
                    Pci::new([network.tap()]).shared(),
                    SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef)
                        .transport(Transport::Tcp)
                        .shared(),
                ]),
                Machine::new([
                    Tcp::new().shared() as SharedProtocol,
                    Ipv4::new(ip_table.clone()).shared(),
                    Pci::new([network.tap()]).shared(),
                    WaitForMessage::new(capture_ip_address, 0xbeef, message.clone())
                        .transport(Transport::Tcp)
                        .disable_checking()
                        .shared(),
                ]),
            ]
            .into_iter()
        })
        .collect();

    let instant = Instant::now();
    run_internet(machines, networks).await;
    println!("{:?}", instant.elapsed());
}
