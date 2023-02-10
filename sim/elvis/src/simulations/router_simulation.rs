use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp, Pci},
    run_internet, Machine, Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const ROUTER_ADDRESS: Ipv4Address = Ipv4Address::new([111, 45, 67, 89]);

/// Simulates a message being forwarded along across many networks.
///

pub async fn router_simulation() {
    let ip_table: IpToTapSlot = 
        [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 1), 
         (IP_ADDRESS_3, 2), (IP_ADDRESS_4, 3)].into_iter().collect();

    let destination = IP_ADDRESS_4.clone();

    let d1 = Capture::new_shared(IP_ADDRESS_2, 0xbeef);
    let d2 = Capture::new_shared(IP_ADDRESS_3, 0xbeeb);
    let d3 = Capture::new_shared(IP_ADDRESS_4, 0xbebe);

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
        Machine::new(
            Pci::new_shared([networks[0].tap()]),
            Router::new_shared()
        ),
        // capture for destination 1
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([networks[1].tap()]),
            d1.clone(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network[2].tap()]),
            d2.clone(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network[3].tap()]),
            d3.clone(),
        ])
    ];

    run_internet(machines, networks).await;
    
    assert_eq!(
        d3.application().message(),
        Some(Message::new("Hello!"))
    );
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_multi() {
        super::router_simulation().await
    }
}