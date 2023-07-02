use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet, IpTable, Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const DESTINATION: Ipv4Address = IP_ADDRESS_2;

// simulates a staticly configured router routing a single packet to one of three destinations
pub async fn router_single() {
    let ip_table: IpTable<Recipient> = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_3, Recipient::with_mac(2, 1)),
        (IP_ADDRESS_4, Recipient::with_mac(3, 1)),
    ]
    .into_iter()
    .collect();

    let dt1: IpTable<Recipient> = [(IP_ADDRESS_2, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt2: IpTable<Recipient> = [(IP_ADDRESS_3, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt3: IpTable<Recipient> = [(IP_ADDRESS_4, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();

    let networks: Vec<_> = (0..4).map(|_| Network::basic()).collect();

    let machines = vec![
        // send message
        new_machine![
            Udp::new(),
            Ipv4::new(
                [(DESTINATION, Recipient::with_mac(0, 1))]
                    .into_iter()
                    .collect(),
            ),
            Pci::new([networks[0].clone()]),
            SendMessage::new(
                vec![Message::new(b"Hello World!")],
                Endpoint {
                    address: DESTINATION,
                    port: 0xbeef,
                },
            ),
        ],
        // machine representing our router
        new_machine![
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone(),
                networks[3].clone(),
            ]),
            Ipv4::new(ip_table.clone()),
            Router::new(ip_table)
        ],
        // capture for destination 1
        new_machine![
            Udp::new(),
            Ipv4::new(dt1),
            Pci::new([networks[1].clone()]),
            Capture::new(
                Endpoint {
                    address: IP_ADDRESS_2,
                    port: 0xbeef,
                },
                1,
            )
        ],
        // capture for destination 2
        new_machine![
            Udp::new(),
            Ipv4::new(dt2),
            Pci::new([networks[2].clone()]),
            Capture::new(
                Endpoint {
                    address: IP_ADDRESS_3,
                    port: 0xbeef,
                },
                1,
            )
        ],
        // capture for destination 3
        new_machine![
            Udp::new(),
            Ipv4::new(dt3),
            Pci::new([networks[3].clone()]),
            Capture::new(
                Endpoint {
                    address: IP_ADDRESS_4,
                    port: 0xbeef,
                },
                1,
            )
        ],
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn router_single() {
        super::router_single().await
    }
}
