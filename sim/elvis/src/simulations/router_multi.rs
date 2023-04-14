use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci, Arp,
    },
    run_internet, Machine, Message, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const IP_ADDRESS_5: Ipv4Address = Ipv4Address::new([123, 45, 67, 93]);

// simulates a message being sent over a network of multiple staticly configured routers
pub async fn router_multi() {
    let destination = IP_ADDRESS_5;

    // The ip table for the first router in path.
    // tells the router which of its tap slots to relay the message to
    let ip_table1: IpToTapSlot = [
        (IP_ADDRESS_1, 0),
        (IP_ADDRESS_2, 1),
        (IP_ADDRESS_3, 1),
        (IP_ADDRESS_4, 2),
        (IP_ADDRESS_5, 2),
    ]
    .into_iter()
    .collect();

    // the ip table for the second router in the path
    let ip_table2: IpToTapSlot = [
        (IP_ADDRESS_1, 0),
        (IP_ADDRESS_4, 0),
        (IP_ADDRESS_2, 1),
        (IP_ADDRESS_3, 2),
        (IP_ADDRESS_5, 0),
    ]
    .into_iter()
    .collect();

    // needed to configure captures
    let dt1: IpToTapSlot = [(IP_ADDRESS_2, 0)].into_iter().collect();
    let dt2: IpToTapSlot = [(IP_ADDRESS_3, 0)].into_iter().collect();
    let dt3: IpToTapSlot = [(IP_ADDRESS_4, 0)].into_iter().collect();
    let dt4: IpToTapSlot = [(IP_ADDRESS_5, 0)].into_iter().collect();

    // configure captures.
    let d1 = Capture::new(IP_ADDRESS_2, 0xbeef, 1).shared();
    let d2 = Capture::new(IP_ADDRESS_3, 0xbeef, 1).shared();
    let d3 = Capture::new(IP_ADDRESS_4, 0xbeef, 1).shared();
    let d4 = Capture::new(IP_ADDRESS_5, 0xbeef, 1).shared();

    let networks = vec![
        Network::basic(),
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
            Arp::new().shared(),
            Pci::new([networks[0].tap()]).shared(),
            SendMessage::new(Message::new(b"Hello World!"), destination, 0xbeef)
                .remote_mac(1)
                .shared(),
        ]),
        // machine representing our router
        Machine::new([
            Pci::new([networks[0].tap(), networks[1].tap(), networks[2].tap()]).shared(),
            Arp::new().shared(),
            Router::new(ip_table1).shared(),
        ]),
        Machine::new([
            Pci::new([networks[1].tap(), networks[3].tap(), networks[4].tap()]).shared(),
            Arp::new().shared(),
            Router::new(ip_table2).shared(),
        ]),
        // capture for destination 1
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt1).shared(),
            Arp::new().shared(),
            Pci::new([networks[3].tap()]).shared(),
            d1.clone(),
        ]),
        // capture for destination 2
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt2).shared(),
            Arp::new().shared(),
            Pci::new([networks[4].tap()]).shared(),
            d2.clone(),
        ]),
        // capture for destination 3
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt3).shared(),
            Arp::new().shared(),
            Pci::new([networks[2].tap()]).shared(),
            d3.clone(),
        ]),
        // capture for destination 4
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(dt4).shared(),
            Arp::new().shared(),
            Pci::new([networks[2].tap()]).shared(),
            d4.clone(),
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
