use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet, Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const IP_ADDRESS_5: Ipv4Address = Ipv4Address::new([123, 45, 67, 93]);
const DESTINATION: Ipv4Address = IP_ADDRESS_5;

// simulates a message being sent over a network of multiple staticly configured routers
pub async fn router_multi() {
    // The ip table for the first router in path.
    // tells the router which of its tap slots to relay the message to
    let ip_table1: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 1)),
        (IP_ADDRESS_2, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_3, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_4, Recipient::with_mac(2, 1)),
        (IP_ADDRESS_5, Recipient::with_mac(2, 2)),
    ]
    .into_iter()
    .collect();

    // the ip table for the second router in the path
    let ip_table2: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 666)),
        (IP_ADDRESS_2, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_3, Recipient::with_mac(2, 1)),
        (IP_ADDRESS_4, Recipient::with_mac(0, 666)),
        (IP_ADDRESS_5, Recipient::with_mac(0, 666)),
    ]
    .into_iter()
    .collect();

    // needed to configure captures
    let dt1: Recipients = [(IP_ADDRESS_2, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt2: Recipients = [(IP_ADDRESS_3, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt3: Recipients = [(IP_ADDRESS_4, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt4: Recipients = [(IP_ADDRESS_5, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();

    let networks: Vec<_> = (0..5).map(|_| Network::basic()).collect();

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
                Endpoint::new(DESTINATION, 0xbeef)
            )
            .process(),
        ],
        // machine representing our router
        new_machine![
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone(),
            ]),
            Ipv4::new(ip_table1.clone()),
            Router::new(ip_table1).process(),
        ],
        new_machine![
            Pci::new([
                networks[1].clone(),
                networks[3].clone(),
                networks[4].clone(),
            ]),
            Ipv4::new(ip_table2.clone()),
            Router::new(ip_table2).process(),
        ],
        // capture for destination 1
        new_machine![Udp::new(), Ipv4::new(dt1), Pci::new([networks[3].clone()]),],
        // capture for destination 2
        new_machine![Udp::new(), Ipv4::new(dt2), Pci::new([networks[4].clone()]),],
        // capture for destination 3
        new_machine![Udp::new(), Ipv4::new(dt3), Pci::new([networks[2].clone()]),],
        // capture for destination 4
        new_machine![
            Udp::new(),
            Ipv4::new(dt4),
            Pci::new([networks[2].clone()]),
            Capture::new(
                Endpoint {
                    address: IP_ADDRESS_5,
                    port: 0xbeef,
                },
                1,
            )
            .process(),
        ],
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn router_multi() {
        super::router_multi().await
    }
}
