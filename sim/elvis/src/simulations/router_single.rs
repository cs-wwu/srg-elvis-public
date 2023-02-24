use std::collections::HashMap;

use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    network::Mac,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);

// simulates a staticly configured router routing a single packet to one of three destinations
pub async fn router_single() {
    // the destination of the capture we want to send the packet to
    let destination = IP_ADDRESS_2.clone();

    let ip_table: IpToTapSlot = [
        (IP_ADDRESS_1, 0),
        (IP_ADDRESS_2, 1),
        (IP_ADDRESS_3, 2),
        (IP_ADDRESS_4, 3),
    ]
    .into_iter()
    .collect();

    let arp_table: HashMap<Ipv4Address, Mac> = [
        (IP_ADDRESS_1, 0),
        (IP_ADDRESS_2, 1),
        (IP_ADDRESS_3, 1),
        (IP_ADDRESS_4, 1),
    ]
    .into_iter()
    .collect();

    let dt1: IpToTapSlot = [(IP_ADDRESS_2, 0)].into_iter().collect();
    let dt2: IpToTapSlot = [(IP_ADDRESS_3, 0)].into_iter().collect();
    let dt3: IpToTapSlot = [(IP_ADDRESS_4, 0)].into_iter().collect();

    let d1 = Capture::new(IP_ADDRESS_2, 0xbeef, 1).shared();
    let d2 = Capture::new(IP_ADDRESS_3, 0xbeef, 1).shared();
    let d3 = Capture::new(IP_ADDRESS_4, 0xbeef, 1).shared();

    let networks = vec![
        Network::basic(),
        Network::basic(),
        Network::basic(),
        Network::basic(),
    ];

    let machines = vec![
        // send message
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new([(destination, 0)].into_iter().collect()).shared(),
            Pci::new([networks[0].tap()]).shared(),
            SendMessage::new(Message::new(b"Hello World!"), destination, 0xbeef)
                .remote_mac(1)
                .shared(),
        ]),
        // machine representing our router
        Machine::new([
            Pci::new([
                networks[0].tap(),
                networks[1].tap(),
                networks[2].tap(),
                networks[3].tap(),
            ])
            .shared(),
            Router::new(ip_table, arp_table).shared(),
        ]),
        // capture for destination 1
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt1).shared(),
            Pci::new([networks[1].tap()]).shared(),
            d1.clone(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt2).shared(),
            Pci::new([networks[2].tap()]).shared(),
            d2.clone(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt3).shared(),
            Pci::new([networks[3].tap()]).shared(),
            d3.clone(),
        ]),
    ];

    run_internet(machines, networks).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn router_simulation() {
        super::router_single().await
    }
}
