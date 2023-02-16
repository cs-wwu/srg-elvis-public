use std::collections::HashMap;

use crate::applications::{Capture, SendMessage, Router};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{ipv4::{Ipv4, Ipv4Address, IpToTapSlot}, udp::Udp, Pci},
    run_internet, Machine, Network, network::Mac,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
// const ROUTER_ADDRESS: Ipv4Address = Ipv4Address::new([111, 45, 67, 89]);

/// Simulates a message being forwarded along across many networks.
///

pub async fn router_single() {
    let ip_table: IpToTapSlot = 
        [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 1), 
         (IP_ADDRESS_3, 2), (IP_ADDRESS_4, 3)].into_iter().collect();

    let arp_table: HashMap<Ipv4Address, Mac> = 
        [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 1), 
         (IP_ADDRESS_3, 1), (IP_ADDRESS_4, 1)].into_iter().collect();

    let dt1:IpToTapSlot = [(IP_ADDRESS_2, 0)].into_iter().collect();
    let dt2:IpToTapSlot = [(IP_ADDRESS_3, 0)].into_iter().collect();
    let dt3:IpToTapSlot = [(IP_ADDRESS_4, 0)].into_iter().collect();

    let destination = IP_ADDRESS_2.clone();

    let d1 = Capture::new_exit_message(IP_ADDRESS_2, 0xbeef, String::from("destination 1"));
    let d2 = Capture::new_exit_message(IP_ADDRESS_3, 0xbeef, String::from("destination 2"));
    let d3 = Capture::new_exit_message(IP_ADDRESS_4, 0xbeef, String::from("destination 3"));

    let networks = vec![
        Network::basic(),
        Network::basic(),
        Network::basic(),
        Network::basic()
    ];
    
    let machines = vec![
        // send message
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(destination, 0)].into_iter().collect()),
            Pci::new_shared([networks[0].tap()]),
            SendMessage::new_shared("Hello!", destination, 0xbeef, Some(1), 1),
        ]),
        // machine representing our router
        Machine::new([
            Pci::new_shared([networks[0].tap(), networks[1].tap(), networks[2].tap(), networks[3].tap()]),
            Router::new_shared(ip_table, arp_table)
        ]),
        // capture for destination 1
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt1),
            Pci::new_shared([networks[1].tap()]),
            d1.clone(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt2),
            Pci::new_shared([networks[2].tap()]),
            d2.clone(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt3),
            Pci::new_shared([networks[3].tap()]),
            d3.clone(),
        ])
    ];

    run_internet(machines, networks).await;

    // println!("{}", d3.application().message().unwrap_or(Message::new(b":(")));
    
    // assert_eq!(
    //     d1.application().message(),
    //     Some(Message::new("Hello!"))
    // );
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn router_simulation() {
        super::router_single().await
    }
}