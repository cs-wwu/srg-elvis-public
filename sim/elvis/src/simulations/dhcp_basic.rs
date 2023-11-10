use crate::{
    applications::{dhcp_server::DhcpServer, Capture, SendMessage},
    ip_generator::IpRange,
};
use elvis_core::{
    new_machine_arc,
    protocols::{
        dhcp::dhcp_client::DhcpClient,
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Arp, Endpoint, Pci,
    },
    run_internet_with_timeout, IpTable, Message, Network, ExitStatus,
};
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

    let status = run_internet_with_timeout(&machines, Duration::from_secs(5)).await;
    assert_eq!(status, ExitStatus::Exited);

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

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn dhcp_basic_offer() {
        for _ in 0..5 {
            super::dhcp_basic_offer().await;
        }
    }
}
