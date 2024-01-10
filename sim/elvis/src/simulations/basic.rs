use std::time::Duration;

use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    message::Message,
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
    let network = Network::basic();
    let message = Message::new("Hello!");
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };

    let local_address: Ipv4Address = [127, 0, 0, 1].into();

    let ip_table: IpTable<Recipient> = [(local_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint),
            Udp::new(),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1),
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
        .protocol::<Capture>()
        .unwrap()
        .message();

    assert_eq!(received, Some(message));
}

/// Runs a basic forward simulation.
///
/// In this simulation, a machine sends a message to another machine with forward which sends to a third machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic_forward() {
    let network = Network::basic();
    let message = Message::new("Hello!");
    let capture_endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };
    let forward_endpoint = Endpoint {
        address: [123, 45, 67, 90].into(),
        port: 0xbeef,
    };

    let local_address: Ipv4Address = [127, 0, 0, 1].into();
    let forward_address: Ipv4Address = [123, 45, 67, 90].into();

    let ip_table: IpTable<Recipient> = [(local_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();
    let forward_ip_table: IpTable<Recipient> = [(forward_address, Recipient::with_mac(0, 2))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], forward_endpoint),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(forward_ip_table.clone()),
            Pci::new([network.clone()]),
            Forward::new(Endpoints::new(forward_endpoint, capture_endpoint)),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(capture_endpoint, 1),
        ],
    ];
    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .nth(2)
        .unwrap()
        .protocol::<Capture>()
        .unwrap()
        .message();

    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn basic() {
        for _ in 0..5 {
            super::basic().await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn basic_forward() {
        for _ in 0..5 {
            super::basic_forward().await;
        }
    }
}
