use std::time::Duration;

use crate::applications::{TcpListenerServer, TcpStreamClient};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

/// Simulation designed to test TcpStream and TcpListener using TcpListenerServer and TcpStreamClient.
pub async fn tcp_stream() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let client_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let server_socket_address: Endpoint = Endpoint::new(server_ip_address, 80);
    let client_socket_address: Endpoint = Endpoint::new(client_ip_address, 70);

    let ip_table: IpTable<Recipient> = [
        (client_ip_address, Recipient::with_mac(0, 0)),
        (server_ip_address, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            TcpStreamClient::new(server_socket_address, client_socket_address),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client_ip_address)),
            TcpListenerServer::new(server_socket_address, client_socket_address),
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_stream() {
        super::tcp_stream().await;
    }
}
