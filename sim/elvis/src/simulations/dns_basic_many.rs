use crate::applications::{dns_test_client::DnsTestClient, dns_test_server::DnsTestServer};
use tokio::time::Duration;
use elvis_core::{
    new_machine_arc,
    protocols::{
        dns::{dns_resolver::DnsResolver, dns_server::DnsServer},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Pci, SocketAPI, Arp,
    },
    IpTable, Network, run_internet_with_timeout,
};

/// Runs a client-server sim using many clients utilizing DNS and a single
/// server to resolve the correct Ipv4 address.
///
/// In this simulation, client machines intend to send a "request" message
/// to a server machine. The client machines only have a name associated with the server in question. The original application will use the local instance 
/// of the DNS protocol to find the Ipv4 address of the intended server by
/// sending a query to the DNS Authoritative server. A client then uses the
/// retrieved Ipv4 address to interact with the non-DNS server.
pub async fn dns_basic_many() {
    let network = Network::basic();
    let dns_server_ip_address = Ipv4Address::DNS_ROOT_AUTH;
    let server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    
    let num_clients: u32 = 10;

    let mut client_ip_addresses: Vec<Ipv4Address> = vec![];

    for i in 0..num_clients {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_client_ip_address = [123, 45, tens, ones].into();  // Ip addresses are arbitrary
        client_ip_addresses.push(this_client_ip_address);
    }

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
    .into_iter()
    .collect();


    let mut machines = vec![];
    
    machines.push(new_machine_arc![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Arp::new(),
        Pci::new([network.clone()]),
        SocketAPI::new(Some(dns_server_ip_address)),
        DnsServer::new()
        ]
    );

    machines.push(new_machine_arc![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Arp::new(),
        Pci::new([network.clone()]),
        SocketAPI::new(Some(server_ip_address)),
        DnsTestServer::new(0xbeef, SocketType::Datagram)
        ]
    );

    for i in 0..num_clients {
        machines.push(new_machine_arc![
                Tcp::new(),
                Udp::new(),
                Ipv4::new(ip_table.clone()),
                Arp::new(),
                Pci::new([network.clone()]),
                SocketAPI::new(Some(client_ip_addresses[i as usize])),
                DnsResolver::new(),
                DnsTestClient::new(0xbeef, SocketType::Datagram),
            ])
    }

    run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn dns_basic_many() {
        super::dns_basic_many().await;
    }
}
