use crate::applications::{
    capture::{CapFactory, Capture},
    SendMessage,
};
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
use std::time::Duration;

/// Simulation to test udp broadcasting and multiple
/// udp sessions on the same machine using the same local ip.
/// Simulates a sendmessage broadcasting to multiple captures
/// on the port 0xbeef. The simulation shuts down once all 3
/// MultiCaptures receive a message
const IPS: [Ipv4Address; 4] = [
    Ipv4Address::new([1, 1, 1, 1]),
    Ipv4Address::new([1, 1, 1, 2]),
    Ipv4Address::new([1, 1, 1, 3]),
    Ipv4Address::new([1, 1, 1, 4]),
];

pub async fn udp_broadcast_basic() -> ExitStatus {
    let network = Network::basic();
    let message = Message::new("Hello!");

    let capfactory = CapFactory::new();

    let endpoint = Endpoint {
        address: [255, 255, 255, 255].into(),
        port: 0xbeef,
    };

    let endpoints: [Endpoint; 4] = [
        Endpoint::new(IPS[0], 0xbeef),
        Endpoint::new(IPS[1], 0xbeef),
        Endpoint::new(IPS[2], 0xbeef),
        Endpoint::new(IPS[3], 0xface),
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
            capfactory.build(endpoints[0], 1).exit_status(1),
            Udp::new(),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            capfactory.build(endpoints[1], 1).exit_status(1),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            capfactory.build(endpoints[2], 1).exit_status(1),
        ],
        // evil machine should not be receiving the udp broadcast
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoints[3], 1).exit_status(2),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(5)).await
}

#[cfg(test)]
mod tests {
    use elvis_core::ExitStatus;

    #[tokio::test(flavor = "multi_thread")]
    async fn udp_broadcast_basic() {
        let status = super::udp_broadcast_basic().await;

        assert_eq!(status, ExitStatus::Status(1));
    }
}
