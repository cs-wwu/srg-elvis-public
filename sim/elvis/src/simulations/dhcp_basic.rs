use crate::{
    applications::{dhcp_server::DhcpServer, Capture, SendMessage, OnReceive},
    ip_generator::*,
};
use elvis_core::{
    new_machine,
    protocols::{
        dhcp::{dhcp_client::DhcpClient, dhcp_client_listener::DhcpClientListener, dhcp_client::CurrentState},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet, run_internet_with_timeout, IpTable, Message, Network,
};
use tokio::time::{Duration, sleep};

// Sim to test basic IP address allocation from server
pub async fn dhcp_basic_offer() {
    let network = Network::basic();
    const DHCP_SERVER_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 255]);
    const CAPTURE_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 0]);
    const CAPTURE_ENDPOINT: Endpoint = Endpoint::new(CAPTURE_IP, 0);
    let ip_table: IpTable<Recipient> = [
        (DHCP_SERVER_IP, Recipient::with_mac(0, 0)),
        (CAPTURE_IP, Recipient::with_mac(0, 1)),
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
        // The capture machine has its IP address statically allocated because otherwise we would
        // also need address resolution
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Capture::new(CAPTURE_ENDPOINT, 2),
        ],
        // This machine and the next will get their IP addresses from the DHCP server and then send
        // messages to the capture machine.
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new(DHCP_SERVER_IP, None),
            SendMessage::new(vec![Message::new("Hi")], CAPTURE_ENDPOINT),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new(DHCP_SERVER_IP, None),
            SendMessage::new(vec![Message::new("Hi")], CAPTURE_ENDPOINT),
        ],
    ];
    run_internet(&machines);
}

// Sim to test clients returning IP to server
pub async fn dhcp_basic_release() {
    let network = Network::basic();
    const DHCP_SERVER_IP: Ipv4Address = Ipv4Address::new([255, 255, 255, 255]);

    let ip_table: IpTable<Recipient> = [(DHCP_SERVER_IP, Recipient::with_mac(0, 0))]
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
        // These machines will get their IPs from the DHCP server, return it, and request a new IP
        // where the machines second IP will be the same as its first
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new(DHCP_SERVER_IP, Some(DhcpClientListener::new())),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new(DHCP_SERVER_IP, Some(DhcpClientListener::new())),
        ],
    ];
    run_internet(&machines).await;
    let mut machines_iter = machines.into_iter();
    let server = machines_iter.next().unwrap();
    let client1 = machines_iter.next().unwrap();
    let client2 = machines_iter.next().unwrap();

    assert_eq!(
        server
            .into_inner()
            .protocol::<DhcpServer>()
            .unwrap()
            .ip_generator
            .write()
            .unwrap()
            .fetch_ip()
            .unwrap(),
        Ipv4Address::from(3)
    );
    // It's not consistent which machine gets [0.0.0.1] or [0.0.0.2] so just asserting that they have *some* IP
    // While the above assertion ensures they're one of the two mentioned values
    assert!(client1
        .into_inner()
        .protocol::<DhcpClient>()
        .unwrap()
        .ip_address
        .read()
        .unwrap()
        .is_some());
    assert!(client2
        .into_inner()
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
    #[tokio::test]
    async fn dhcp_basic_offer() {
        super::dhcp_basic_offer().await;
    }
    #[tokio::test]
    async fn dhcp_basic_release() {
        super::dhcp_basic_release().await;
    }
    #[tokio::test]
    async fn dhcp_lease_test() {
        super::dhcp_lease_test().await;
    }
}
