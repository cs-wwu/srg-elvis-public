use crate::applications::{SendMessage, WaitForMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Recipient},
        Endpoint, Pci, Tcp,
    },
    run_internet, IpTable, Transport,
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
    let ip_table: IpTable<Recipient> = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..1_000_000_000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint).transport(Transport::Tcp)
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            WaitForMessage::new(endpoint, message)
                .transport(Transport::Tcp)
                .disable_checking()
        ],
    ];

    run_internet(&machines).await;
}
