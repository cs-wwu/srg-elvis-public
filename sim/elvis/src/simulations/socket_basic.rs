use crate::applications::{SocketClient, SocketServer, ArpRouter};
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
    run_internet, IpTable, Network, machine::PciSlot,
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

    let router_ip: Ipv4Address = [1,1,1,1].into();

    let router_table: IpTable<(Option<Ipv4Address>, PciSlot)> =
        [("123.45.67.0/24", (None, 0))].into_iter().collect();

    let mut router_ips: Vec<Ipv4Address> = Vec::new();
    router_ips.push(router_ip);

    let ip_table: IpTable<Recipient> = [
        (server_ip_address, Recipient::new(0, None)),
        (client1_ip_address, Recipient::new(0, None)),
        (client2_ip_address, Recipient::new(0, None)),
        (client3_ip_address, Recipient::new(0, None)),
    ]
    .into_iter()
    .collect();

    let router_ip_table: IpTable<Recipient> = [
        (router_ip, Recipient::new(0, None))
    ].into_iter().collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::basic().preconfig_subnet(
                router_ip,
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: router_ip
                }
            ),
            SocketAPI::new(Some(server_ip_address)),
            SocketServer::new(0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::basic().preconfig_subnet(
                router_ip,
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: router_ip
                }
            ),
            SocketAPI::new(Some(client1_ip_address)),
            SocketClient::new(1, server_ip_address, 0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::basic().preconfig_subnet(
                router_ip,
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: router_ip
                }
            ),
            SocketAPI::new(Some(client2_ip_address)),
            SocketClient::new(2, server_ip_address, 0xbeef, SocketType::Stream)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::basic().preconfig_subnet(
                router_ip,
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: router_ip
                }
            ),
            SocketAPI::new(Some(client3_ip_address)),
            SocketClient::new(3, server_ip_address, 0xbeef, SocketType::Stream)
        ],
        new_machine![
            Pci::new([network.clone()]),
            Ipv4::new(router_ip_table),
            Arp::basic(),
            ArpRouter::new(router_table, router_ips)
        ]
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn socket_basic() {
        super::socket_basic().await;
    }
}
