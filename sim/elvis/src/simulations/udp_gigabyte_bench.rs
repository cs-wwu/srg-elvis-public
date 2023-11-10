use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, Udp,
    },
    run_internet, IpTable,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn udp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let endpoint = Endpoint::new([123, 45, 67, 89].into(), 0xbeef);
    let ip_table: IpTable<Recipient> =
        [(Ipv4Address::from([127, 0, 0, 1]), Recipient::with_mac(0, 1))]
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

    let count: u32 =
        u32::try_from(messages.len()).expect("there should be less than 4 billion messages");

    let machines = vec![
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(messages, endpoint)
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoint, count),
        ],
    ];

    run_internet(&machines).await;
}
