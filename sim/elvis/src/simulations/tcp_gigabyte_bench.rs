use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    message::Message,
    network::NetworkBuilder,
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        Endpoint, Pci, Tcp,
    },
    run_internet, Machine, Transport,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..1_000_000_000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Tcp::new())
                .with(Ipv4::new(ip_table.clone()))
                .with(Pci::new([network.clone()]))
                .with(
                    SendMessage::new(vec![message.clone()], endpoint)
                        .transport(Transport::Tcp)
                        .process(),
                )
                .build(),
        ),
        Machine::new(
            ProtocolMapBuilder::new()
                .with(Tcp::new())
                .with(Ipv4::new(ip_table))
                .with(Pci::new([network.clone()]))
                .with(
                    WaitForMessage::new(endpoint, message)
                        .transport(Transport::Tcp)
                        .disable_checking()
                        .process(),
                )
                .build(),
        ),
    ];

    run_internet(&machines).await;
}
