use crate::applications::{
    web_server::{WebServer, WebServerType},
    SimpleWebClient,
};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, IpTable, Network,
};
use std::time::Duration;

/// Simulation designed to test WebServer using SimpleWebClient
pub async fn yahoo_server() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [100, 42, 0, 0].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let client2_ip_address: Ipv4Address = [123, 45, 67, 91].into();
    let client3_ip_address: Ipv4Address = [123, 45, 67, 92].into();
    let server_socket_address: Endpoint = Endpoint::new(server_ip_address, 80);

    let ip_table: IpTable<Recipient> = [
        (server_ip_address, Recipient::with_mac(0, 0)),
        (client1_ip_address, Recipient::with_mac(0, 1)),
        (client2_ip_address, Recipient::with_mac(0, 1)),
        (client3_ip_address, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            WebServer::new(WebServerType::Yahoo, Some(13)),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client1_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client2_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client3_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(3)).await;

    let mut machines_iter = machines.into_iter();
    let _server = machines_iter.next().unwrap();

    // Check that each client recieved at least 1000 pages before the simulation was terminated
    for _i in 0..3 {
        let client = machines_iter.next().unwrap();
        let lock = &client
            .into_inner()
            .protocol::<SimpleWebClient>()
            .unwrap()
            .num_pages_recvd;
        let num_pages_recvd = *lock.read().unwrap();
        assert!(num_pages_recvd > 500)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn yahoo_server() {
        super::yahoo_server().await;
    }
}
