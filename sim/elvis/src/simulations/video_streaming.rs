use crate::applications::{streaming_client::StreamingClient, streaming_server::VideoServer};

use elvis_core::{
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, IpTable, Network, ExitStatus,
};
use std::time::Duration;

/**
 * Runs a basic video server and client simulation.
 *
 * In this simulation, a client requests video data from a streaming server, which
 * then sends the data to the client. The client will then "play" the video in the form
 * of printing the bytes it recieved to the terminal (I've commented it out, but it can be
 * uncommented for testing). The sim currently ends when it times out
 * via a specified duration.
 */
pub async fn video_streaming() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [100, 42, 0, 0].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let client2_ip_address: Ipv4Address = [123, 45, 67, 91].into();
    let client3_ip_address: Ipv4Address = [123, 45, 67, 92].into();
    let server_socket_address: Endpoint = Endpoint::new(server_ip_address, 80);

    let ip_table: IpTable<Recipient> = [
        (server_ip_address, Recipient::with_mac(0, 1)),
        (client1_ip_address, Recipient::with_mac(0, 0)),
        (client2_ip_address, Recipient::with_mac(0, 0)),
        (client3_ip_address, Recipient::with_mac(0, 0)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        // server #1
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            VideoServer::new(server_socket_address),
        ],
        // client #1
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client1_ip_address)),
            StreamingClient::new(server_socket_address),
        ],
        // client #2
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client2_ip_address)),
            StreamingClient::new(server_socket_address),
        ],
        // client #3
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client3_ip_address)),
            StreamingClient::new(server_socket_address),
        ],
    ];

    let duration = 5;
    let status = run_internet_with_timeout(&machines, Duration::from_secs(duration)).await;
    assert_eq!(status, ExitStatus::Exited);

    let mut machines_iter = machines.into_iter();
    let _server = machines_iter.next().unwrap();

    // check that the client received the minimum number of bytes before terminating
    for _i in 0..3 {
        let client = machines_iter.next().unwrap();
        let lock = &client.protocol::<StreamingClient>().unwrap().bytes_recieved;
        let num_bytes_recvd = *lock.read().unwrap();

        // min bytes sent to each client should be
        // duration * low bitrate segment (currently 10)
        let target_bytes = 10 * duration;
        assert!(num_bytes_recvd >= target_bytes.try_into().unwrap());
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    pub async fn video_streaming() {
        for _ in 0..5 {
            super::video_streaming().await;
        }
    }
}
