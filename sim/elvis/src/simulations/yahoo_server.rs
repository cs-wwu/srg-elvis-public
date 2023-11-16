use crate::applications::{
    web_server::{WebServer, WebServerType},
    SimpleWebClient,
};
use elvis_core::{
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Arp, Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
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

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(server_ip_address)),
            WebServer::new(WebServerType::Yahoo, Some(13)),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client1_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client2_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client3_ip_address)),
            SimpleWebClient::new(server_socket_address),
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);

    let mut machines_iter = machines.into_iter();
    let _server = machines_iter.next().unwrap();

    // Check that each client recieved at least 500 pages before the simulation was terminated
    for _i in 0..3 {
        let client = machines_iter.next().unwrap();
        let lock = &client
            .protocol::<SimpleWebClient>()
            .unwrap()
            .num_pages_recvd;
        let num_pages_recvd = *lock.read().unwrap();
        assert!(num_pages_recvd > 500)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn yahoo_server() {
        for _ in 0..5 {
            super::yahoo_server().await;
        }
    }
}
