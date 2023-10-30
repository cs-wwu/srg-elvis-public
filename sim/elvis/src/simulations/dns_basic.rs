use std::time::Duration;

use crate::applications::{dns_test_client::DnsTestClient, dns_test_server::DnsTestServer};
use elvis_core::{
    new_machine,
    protocols::{
        dns::{dns_client::DnsClient, dns_server::DnsServer},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Arp, Pci, SocketAPI,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

/// Runs a basic client-server sim using the DNS client and server to resolve
/// the correct Ipv4 address.
///
/// In this simulation, a client machine intends to send a "request" messages
/// to a server machine. The client machine only has a name associated with the
/// server in question. The original application will use the local instance of
/// the DNS protocol to find out the Ipv4 address of the intended server by
/// sending a query to the DNS Authoritative server. The client then uses the
/// retrieved Ipv4 address to interact with the non-DNS server.
pub async fn dns_basic() {
    let network = Network::basic();
    let dns_server_ip_address = Ipv4Address::DNS_AUTH;
    let server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 60].into();
    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Arp::new(),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(dns_server_ip_address)),
            DnsServer::new(1), // Argument is for number of connections this server will at most have open. WiP workaround solution for now.
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Arp::new(),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_address)),
            DnsTestServer::new(0xbeef, SocketType::Datagram)
        ],
        new_machine![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Arp::new(),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client1_ip_address)),
            DnsClient::new(),
            DnsTestClient::new(0xbeef, SocketType::Datagram)
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Status(10));
}

#[cfg(test)]
mod tests {

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn dns_basic() {
        for _ in 0..5 {
            super::dns_basic().await;
        }
    }
}
