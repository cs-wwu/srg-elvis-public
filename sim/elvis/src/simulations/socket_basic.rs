use std::time::Duration;

use crate::applications::{SocketClient, SocketServer};
use elvis_core::{
    new_machine,
    protocols::{
        arp::subnetting::{Ipv4Mask, SubnetInfo},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Arp, Pci, SocketAPI,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

/// Runs a basic server-client simulation using sockets.
///
/// In this simulation, three client machines send "request" messages to a
/// server machine. The server receives the requests, and sends back
/// "response" messages to each client. The clients receive those
/// responses, and each send back an "ackowledgement" message. The server
/// receives the "ackowledgement" messages, and shuts down the simulation.
pub async fn socket_basic() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let client2_ip_address: Ipv4Address = [123, 45, 67, 91].into();
    let client3_ip_address: Ipv4Address = [123, 45, 67, 92].into();

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let info = SubnetInfo {
        mask: Ipv4Mask::from_bitcount(0),
        default_gateway: Ipv4Address::from([1, 1, 1, 1]),
    };

    let machines = vec![
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet(server_ip_address, info),
            SocketAPI::new(Some(server_ip_address)),
            SocketServer::new(0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet(client1_ip_address, info),
            SocketAPI::new(Some(client1_ip_address)),
            SocketClient::new(1, server_ip_address, 0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet(client2_ip_address, info),
            SocketAPI::new(Some(client2_ip_address)),
            SocketClient::new(2, server_ip_address, 0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet(client3_ip_address, info),
            SocketAPI::new(Some(client3_ip_address)),
            SocketClient::new(3, server_ip_address, 0xbeef, SocketType::Stream)
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn socket_basic() {
        super::socket_basic().await;
    }
}
