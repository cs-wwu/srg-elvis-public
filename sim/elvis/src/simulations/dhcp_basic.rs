use crate::applications::{DhcpClient, DhcpServer};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci, Sockets,
    },
    run_internet, Machine, Network,
};
use std::sync::Arc;
use tokio::sync::Barrier;

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

    let server_socket = Sockets::new(Some(server_ip)).shared();
    let client1_socket = Sockets::new(Some(client1_ip)).shared();
    let client2_socket = Sockets::new(Some(client2_ip)).shared();
    let client3_socket = Sockets::new(Some(client3_ip)).shared();
    let shutdown_bar = Arc::new(Barrier::new(3));
    let machines = vec![
        // Server
        Machine::new([
            server_socket.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            DhcpServer::new(server_socket).shared(),
        ]),
        // Client
        Machine::new([
            client1_socket.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            DhcpClient::new(client1_socket, shutdown_bar.clone(), false).shared(),
        ]),
        Machine::new([
            client2_socket.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            DhcpClient::new(client2_socket, shutdown_bar.clone(), false).shared(),
        ]),
        Machine::new([
            client3_socket.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            DhcpClient::new(client3_socket, shutdown_bar.clone(), true).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn dhcp_basic() {
        super::dhcp_basic().await;
    }
}
