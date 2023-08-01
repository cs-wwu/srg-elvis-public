use std::collections::BTreeMap;

use crate::applications::{dns_test_client::DnsTestClient, dns_test_server::DnsTestServer};
use tokio::time::Duration;
use elvis_core::{
    new_machine,
    protocols::{
        dns::{dns_client::DnsClient, dns_server::DnsServer},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Pci, SocketAPI,
    },
    IpTable, Network, run_internet_with_timeout,
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
pub async fn dns_basic_many() {
    let network = Network::basic();
    let dns_server_ip_address = Ipv4Address::DNS_AUTH;
    let server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    
    let num_clients: u32 = 999;
    // let num_servers: u32 = 1;

    let mut client_ip_addresses: Vec<Ipv4Address> = vec![];

    let mut ip_map = BTreeMap::new();

    for i in 0..num_clients {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_client_ip_address = [123, 45, tens, ones].into();  // Ip addresses are arbitrary
        client_ip_addresses.push(this_client_ip_address);
        ip_map.insert(this_client_ip_address, Recipient::with_mac(0, 1));
    }

    ip_map.insert(dns_server_ip_address, Recipient::with_mac(0, 0));
    ip_map.insert(server_ip_address, Recipient::with_mac(0, 1));

    let ip_table: IpTable<Recipient> = ip_map.into_iter().collect();


    let mut machines = vec![];
    
    machines.push(new_machine![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Pci::new([network.clone()]),
        SocketAPI::new(Some(dns_server_ip_address)),
        DnsServer::new()
        ]
    );

    machines.push(new_machine![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Pci::new([network.clone()]),
        SocketAPI::new(Some(server_ip_address)),
        DnsTestServer::new(0xbeef, SocketType::Datagram)
        ]
    );

    for i in 0..num_clients {
        // let server_index = i % num_servers;
        // println!("server index: {}", server_index);
        machines.push(new_machine![
                Tcp::new(),
                Udp::new(),
                Ipv4::new(ip_table.clone()),
                Pci::new([network.clone()]),
                SocketAPI::new(Some(client_ip_addresses[i as usize])),
                DnsClient::new(),
                DnsTestClient::new(0xbeef, SocketType::Datagram),
            ])
    }

    run_internet_with_timeout(&machines, Duration::from_secs(5)).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn dns_basic_many() {
        super::dns_basic_many().await;
    }
}
