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
const IP_ADDRESS_5: Ipv4Address = Ipv4Address::new([123, 45, 67, 93]);

// simulates a message being sent over a network of multiple staticly configured routers
pub async fn router_multi() {
    let destination = IP_ADDRESS_5.clone();

    // The ip table for the first router in path. 
    // tells the router which of its tap slots to relay the message to
    let ip_table1: IpToTapSlot = 
        [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 1), 
         (IP_ADDRESS_3, 1), (IP_ADDRESS_4, 2),
         (IP_ADDRESS_5, 2)].into_iter().collect();

    // the arp table for the first router in path
    // tells the router which machine on the destination tap slot
    // to send the message to.
    let arp_table1: HashMap<Ipv4Address, Mac> = 
        [(IP_ADDRESS_1, 1), (IP_ADDRESS_2, 1), 
         (IP_ADDRESS_3, 1), (IP_ADDRESS_4, 1),
         (IP_ADDRESS_5, 2)].into_iter().collect();

    // the ip table for the second router in the path
    let ip_table2: IpToTapSlot =
        [(IP_ADDRESS_1, 0), (IP_ADDRESS_4, 0), 
         (IP_ADDRESS_2, 1), (IP_ADDRESS_3, 2),
         (IP_ADDRESS_5, 0)].into_iter().collect();
    
    // the arp table for the second router in the path
    let arp_table2: HashMap<Ipv4Address, Mac> =
        [(IP_ADDRESS_2, 1), (IP_ADDRESS_3, 1)].into_iter().collect();

    // needed to configure captures
    let dt1:IpToTapSlot = [(IP_ADDRESS_2, 0)].into_iter().collect();
    let dt2:IpToTapSlot = [(IP_ADDRESS_3, 0)].into_iter().collect();
    let dt3:IpToTapSlot = [(IP_ADDRESS_4, 0)].into_iter().collect();
    let dt4:IpToTapSlot = [(IP_ADDRESS_5, 0)].into_iter().collect();

    // configure captures.
    let d1 = Capture::new_exit_message(
        IP_ADDRESS_2, 0xbeef, String::from("destination 1")
    );
    let d2 = Capture::new_exit_message(
        IP_ADDRESS_3, 0xbeef, String::from("destination 2")
    );
    let d3 = Capture::new_exit_message(
        IP_ADDRESS_4, 0xbeef, String::from("destination 3")
    );
    let d4 = Capture::new_exit_message(
        IP_ADDRESS_5, 0xbeef, String::from("destination 4")
    );

    let networks = vec![
        Network::basic(),
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
            SendMessage::new_shared("Hello World!", destination, 0xbeef, Some(1), 1),
        ]),
        // machine representing our router
        Machine::new([
            Pci::new_shared([networks[0].tap(), networks[1].tap(), networks[2].tap()]),
            Router::new_shared(ip_table1, arp_table1)
        ]),
        Machine::new([
            Pci::new_shared([networks[1].tap(), networks[3].tap(), networks[4].tap()]),
            Router::new_shared(ip_table2, arp_table2)
        ]),
        // capture for destination 1
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt1),
            Pci::new_shared([networks[3].tap()]),
            d1.clone(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt2),
            Pci::new_shared([networks[4].tap()]),
            d2.clone(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt3),
            Pci::new_shared([networks[2].tap()]),
            d3.clone(),
        ]),
        // capture for destination 4
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(dt4),
            Pci::new_shared([networks[2].tap()]),
            d4.clone(),
        ])
    ];

    run_internet(machines, networks).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn router_multi() {
        super::router_multi().await
    }
}