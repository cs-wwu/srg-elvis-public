use crate::applications::{streaming_server::VideoServer, streaming_client::StreamingClient};

use std::time::Duration;
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, SocketAPI, Tcp,
    },
    IpTable, Network, run_internet_with_timeout,
};

/**
 * Runs a basic video server and client simulation.
 * 
 * In this simulation, a client requests video data from the server, which
 * then sends the data to the client. The sim currently ends when it times out
 * via a specified duration in streaming_server.
 */
pub async fn video_streaming() {
    //let handle_server = std::thread::spawn(|| streaming_server::server());
    //let handle_client = std::thread::spawn(|| streaming_client::client());    

    //handle_client.join().unwrap();
    //handle_server.join().unwrap();
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [100, 42, 0, 1].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let server_socket_address: Endpoint = Endpoint::new(server_ip_address, 80);

    let ip_table: IpTable<Recipient> = [
        (server_ip_address, Recipient::with_mac(0, 0)),
        (client1_ip_address, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        // server #1
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            VideoServer::new(),
        ],
        // client #1
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client1_ip_address)),
            StreamingClient::new(server_socket_address),
        ],
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    //println!("Running internet with timeout...");
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn video_streaming() {
        println!("Running video streaming test...");
        super::video_streaming().await;
    }
}