use crate::applications::{dns_test_client::DnsTestClient, dns_test_server::DnsTestServer};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Pci, SocketAPI,
        dns::{dns_client::DnsClient, dns_server::DnsServer}
    },
    run_internet, Network, IpTable,
};

/// Runs a basic server-client simulation using sockets.
///
/// In this simulation, three client machines send "request" messages to a
/// server machine. The server receives the requests, and sends back
/// "response" messages to each client. The clients receive those
/// responses, and each send back an "ackowledgement" message. The server
/// receives the "ackowledgement" messages, and shuts down the simulation.
pub async fn dns_basic() {
    let network = Network::basic();
    let dns_server_ip_address = Ipv4Address::DNS_AUTH;
    let server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 60].into();
    let ip_table: IpTable<Recipient> = [
        (dns_server_ip_address, Recipient::with_mac(0, 0)),
        (server_ip_address, Recipient::with_mac(0, 1)),
        (client1_ip_address, Recipient::with_mac(0, 2)),
    ]
    .into_iter()
    .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(dns_server_ip_address)),
            DnsServer::new(1),  // Argument is for number of connections this server will at most have open. WiP workaround solution for now.
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            DnsTestServer::new(0xbeef, SocketType::Datagram)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client1_ip_address)),
            DnsClient::new(),
            DnsTestClient::new(0xbeef, SocketType::Datagram)
        ],
    ];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn dns_basic() {
        super::dns_basic().await;
    }
}
