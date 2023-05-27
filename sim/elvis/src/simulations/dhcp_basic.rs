use crate::applications::dhcp::{dhcp_client::DhcpClient, dhcp_server::DhcpServer};
use elvis_core::{
    machine::ProtocolMapBuilder,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci, Sockets,
    },
    run_internet, Machine, Network,
};

pub async fn dhcp_basic() {
    let network = Network::basic();
    let server_ip: Ipv4Address = [255, 255, 255, 255].into();
    let client1_ip: Ipv4Address = [0, 0, 0, 0].into();
    let client2_ip: Ipv4Address = [0, 0, 0, 1].into();
    let client3_ip: Ipv4Address = [0, 0, 0, 2].into();
    let ip_table: Recipients = [
        (server_ip, Recipient::with_mac(0, 0)),
        (client1_ip, Recipient::with_mac(0, 1)),
        (client2_ip, Recipient::with_mac(0, 2)),
        (client3_ip, Recipient::with_mac(0, 3)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        // Server
        new_machine![
            Sockets::new(Some(server_ip)),
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpServer::new().process(),
        ],
        // Client
        new_machine![
            Sockets::new(Some(client1_ip)),
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new().process(),
        ],
        new_machine![
            Sockets::new(Some(client2_ip)),
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new().process(),
        ],
        new_machine![
            Sockets::new(Some(client3_ip)),
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            DhcpClient::new().process(),
        ],
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn dhcp_basic() {
        super::dhcp_basic().await;
    }
}
