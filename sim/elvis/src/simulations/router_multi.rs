use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Arp, Pci,
    },
    run_internet, Machine, Message, Network,
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
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(
                [(DESTINATION, Recipient::with_mac(0, 1))]
                    .into_iter()
                    .collect(),
            )
            .shared(),
            Pci::new([networks[0].clone()]).shared(),
            SendMessage::new(vec![Message::new(b"Hello World!")], DESTINATION, 0xbeef).shared(),
        ]),
        // machine representing our router
        Machine::new([
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone(),
            ])
            .shared() as SharedProtocol,
            Router::new(ip_table1).shared(),
        ]),
        Machine::new([
            Pci::new([
                networks[1].clone(),
                networks[3].clone(),
                networks[4].clone(),
            ])
            .shared() as SharedProtocol,
            Router::new(ip_table2).shared(),
        ]),
        // capture for destination 1
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt1).shared(),
            Pci::new([networks[3].clone()]).shared(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt2).shared(),
            Pci::new([networks[4].clone()]).shared(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt3).shared(),
            Pci::new([networks[2].clone()]).shared(),
        ]),
        // capture for destination 4
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt4).shared(),
            Pci::new([networks[2].clone()]).shared(),
            Capture::new(IP_ADDRESS_5, 0xbeef, 1).shared(),
        ]),
    ];

    run_internet(machines, networks).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn router_multi() {
        super::router_multi().await
    }
}
