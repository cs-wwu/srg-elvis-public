use crate::applications::PingPong;
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Endpoint, Endpoints, Pci,
    },
    run_internet, Network,
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
    let ip_table: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            PingPong::new(true, endpoints).process()
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            PingPong::new(false, endpoints.reverse()).process()
        ],
    ];

    run_internet(&machines).await;

    // TODO(hardint): Should check here that things actually ran correctly
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn ping_pong() {
        super::ping_pong().await
    }
}
