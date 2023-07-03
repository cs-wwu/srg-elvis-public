use std::time::Duration;

use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout,
    shutdown::ExitStatus,
    Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const ROUTER_IP: Ipv4Address = Ipv4Address::new([123, 45, 76, 92]);

// simulates a staticly configured router routing a single packet to one of three destinations
pub async fn router_single(destination: Ipv4Address) -> ExitStatus {
    let ip_table: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_3, Recipient::with_mac(2, 1)),
        (IP_ADDRESS_4, Recipient::with_mac(3, 1)),
    ]
    .into_iter()
    .collect();

    let dt1: Recipients = [(IP_ADDRESS_2, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();
    let dt2: Recipients = [(IP_ADDRESS_3, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();
    let dt3: Recipients = [(IP_ADDRESS_4, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let networks: Vec<_> = (0..4).map(|_| Network::basic()).collect();

    let machines = vec![
        // send message
        new_machine![
            Udp::new(),
            Ipv4::new(
                [(destination, Recipient::with_mac(0, 1))]
                    .into_iter()
                    .collect(),
            ),
            Pci::new([networks[0].clone()]),
            SendMessage::new(
                vec![Message::new(b"Hello World!")],
                Endpoint {
                    address: destination,
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
            Router::new(ip_table, ROUTER_IP)
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
            .exit_status(1),
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
            .exit_status(2),
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
            .exit_status(3),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(2)).await
}

#[cfg(test)]
mod tests {
    use elvis_core::protocols::ipv4::Ipv4Address;

    const EVIL: Ipv4Address = Ipv4Address::new([123, 45, 67, 93]);
    #[tokio::test]
    async fn router_single() {
        let test1 = super::router_single(super::IP_ADDRESS_2);
        let test2 = super::router_single(super::IP_ADDRESS_3);
        let test3 = super::router_single(super::IP_ADDRESS_4);
        let test4 = super::router_single(EVIL);

        assert_eq!(test1.await, super::ExitStatus::Status(1));
        assert_eq!(test2.await, super::ExitStatus::Status(2));
        assert_eq!(test3.await, super::ExitStatus::Status(3));
        assert_eq!(test4.await, super::ExitStatus::TimedOut);
    }
}
