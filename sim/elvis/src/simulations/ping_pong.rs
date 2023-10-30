use std::time::Duration;

use crate::applications::PingPong;
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);

/// Runs a basic PingPong simulation.
///
/// In this simulation, two machines will send a Time To Live (TTL) message
/// back and forth till the TTL reaches 0. TTL will be subtracted by 1 every time a machine reveives it.
pub async fn ping_pong() {
    let network = Network::basic();
    let endpoints = Endpoints {
        local: Endpoint {
            address: IP_ADDRESS_1,
            port: 0xbeef,
        },
        remote: Endpoint {
            address: IP_ADDRESS_2,
            port: 0xface,
        },
    };

    let ip_table: IpTable<Recipient> = [
        (IP_ADDRESS_2, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_1, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            PingPong::new(true, endpoints)
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            PingPong::new(false, endpoints.reverse())
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);

    // TODO(hardint): Should check here that things actually ran correctly
}

#[cfg(test)]
mod tests {

    #[tokio::test(flavor = "multi_thread")]
    pub async fn ping_pong() {
        for _ in 0..5 {
            super::ping_pong().await;
        }
    }
}
