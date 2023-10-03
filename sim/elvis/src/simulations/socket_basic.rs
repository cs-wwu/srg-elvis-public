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
/// In this simulation, several client machines send "request" messages to a
/// server machine. The server receives the requests, and sends back
/// "response" messages to each client. The clients receive those
/// responses, and each send back an "ackowledgement" message. The server
/// receives the "ackowledgement" messages, and shuts down the simulation.
pub async fn socket_basic(transport: SocketType, num_clients: u8) {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [111, 111, 11, 0].into();

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let info = SubnetInfo {
        mask: Ipv4Mask::from_bitcount(0),
        default_gateway: Ipv4Address::from([1, 1, 1, 1]),
    };

    let mut machines = vec![new_machine![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Pci::new([network.clone()]),
        Arp::new().preconfig_subnet(server_ip_address, info),
        SocketAPI::new(Some(server_ip_address)),
        SocketServer::new(0xbeef, transport, num_clients.into())
    ]];
    for i in 1..=num_clients {
        machines.push(new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet([111, 111, 11, i].into(), info),
            SocketAPI::new(Some([111, 111, 11, i].into())),
            SocketClient::new(i.into(), server_ip_address, 0xbeef, transport)
        ])
    }

    println!("Number of machines: {:?}", machines.len());

    let status = run_internet_with_timeout(&machines, Duration::from_secs(10)).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    use elvis_core::protocols::socket_api::socket::SocketType;

    #[tokio::test]
    async fn socket_basic_tcp() {
        super::socket_basic(SocketType::Stream, 1).await;
    }

    #[tokio::test]
    async fn socket_basic_udp() {
        super::socket_basic(SocketType::Datagram, 1).await;
    }

    #[tokio::test]
    async fn socket_basic_tcp_10_clients() {
        super::socket_basic(SocketType::Stream, 10).await;
    }

    #[tokio::test]
    async fn socket_basic_udp_10_clients() {
        super::socket_basic(SocketType::Datagram, 10).await;
    }

    #[tokio::test]
    async fn socket_basic_tcp_100_clients() {
        super::socket_basic(SocketType::Stream, 100).await;
    }

    #[tokio::test]
    async fn socket_basic_udp_100_clients() {
        super::socket_basic(SocketType::Datagram, 100).await;
    }
}
