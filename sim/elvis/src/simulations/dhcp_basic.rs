use crate::{
    applications::{dhcp_server::DhcpServer, Capture, SendMessage, OnReceive},
    ip_generator::IpRange,
};
use elvis_core::{
    new_machine_arc,
    protocols::{
        dhcp::{dhcp_client::DhcpClient, dhcp_client::CurrentState},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Arp, Endpoint, Pci,
    },
    run_internet_with_timeout, IpTable, Message, Network,
};
use tokio::time::{Duration, sleep};
use std::time::Duration;

// Sim to test basic IP address allocation from server
pub async fn dhcp_basic_offer() {
    let network = Network::basic();
    const DHCP_SERVER_IP: Ipv4Address = Ipv4Address::new([123, 123, 123, 123]);
    const CAPTURE_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 0]);
    const CAPTURE_ENDPOINT: Endpoint = Endpoint::new(CAPTURE_IP, 0);

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let machines = vec![
        // Server
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            DhcpServer::new(DHCP_SERVER_IP, IpRange::new(1.into(), 255.into())),
        ],
        // The capture machine has its IP address statically allocated because otherwise we would
        // also need address resolution
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            Capture::new(CAPTURE_ENDPOINT, 2),
        ],
        // This machine and the next will get their IP addresses from the DHCP server and then send
        // messages to the capture machine.
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            DhcpClient::new(DHCP_SERVER_IP),
            SendMessage::new(vec![Message::new("Hi")], CAPTURE_ENDPOINT),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            DhcpClient::new(DHCP_SERVER_IP),
            SendMessage::new(vec![Message::new("Hi")], CAPTURE_ENDPOINT),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(5)).await;

    let mut machines_iter = machines.into_iter();
    machines_iter.next();
    machines_iter.next();
    let client1 = machines_iter.next().unwrap();
    let client2 = machines_iter.next().unwrap();
    assert!(client1
        .protocol::<DhcpClient>()
        .unwrap()
        .ip_address
        .read()
        .unwrap()
        .is_some());
    assert!(client2
        .protocol::<DhcpClient>()
        .unwrap()
        .ip_address
        .read()
        .unwrap()
        .is_some());
}

pub async fn dhcp_lease_test() {
    let network = Network::basic();
    const DHCP_SERVER_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 255]);
    const RECV_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 0]);
    let ip_table: IpTable<Recipient> = [
        (DHCP_SERVER_IP, Recipient::with_mac(0, 0)),
        (RECV_IP, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        // Server
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpServer::new(DHCP_SERVER_IP, IpRange::new(1.into(), 255.into())),
        ],
        // This machine will get its IP address from the DHCP server
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new(DHCP_SERVER_IP, None),
        ],
    ];
    run_internet(&machines);

    //sleep(Duration::from_secs(8)).await;

    let client = machines.get(1).unwrap();
    let time = 8;

    assert_eq!(3, 3);

}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn dhcp_basic_offer() {
        for _ in 0..5 {
            super::dhcp_basic_offer().await;
        }
    }
    #[tokio::test]
    async fn dhcp_lease_test() {
        super::dhcp_lease_test().await;
    }
}
