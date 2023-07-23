use std::{sync::Arc, time::Duration};

use crate::applications::{Counter, MultiCapture, SendMessage};
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
///

const IPS: [Ipv4Address; 4] = [
    Ipv4Address::new([1, 1, 1, 1]),
    Ipv4Address::new([1, 1, 1, 2]),
    Ipv4Address::new([1, 1, 1, 3]),
    Ipv4Address::new([1, 1, 1, 4])
];

pub async fn udp_broadcast_basic() -> ExitStatus {
    let network = Network::basic();
    let message = Message::new("Hello!");

    let counter: Arc<Counter> = Counter::new(4);

    let endpoint = Endpoint {
        address: [255, 255, 255, 255].into(),
        port: 0xbeef,
    };

    let endpoints: [Endpoint; 4] = [
        Endpoint::new(IPS[0], 0xbeef),
        Endpoint::new(IPS[1], 0xbeef),
        Endpoint::new(IPS[2], 0xbeef),
        Endpoint::new(IPS[3], 0xbeef),
    ];

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint),
            MultiCapture::new(endpoints[0], counter.clone()).exit_status(1),
            Udp::new(),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            MultiCapture::new(endpoints[1], counter.clone()).exit_status(1),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            MultiCapture::new(endpoints[2], counter.clone()).exit_status(1),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            MultiCapture::new(endpoints[3], counter.clone()).exit_status(1),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(5)).await
}

#[cfg(test)]
mod tests {
    use elvis_core::ExitStatus;

    #[tokio::test]
    async fn udp_broadcast_basic() {
        let status = super::udp_broadcast_basic().await;

        assert_eq!(status, ExitStatus::Status(1));
    }
}
