use crate::applications::{TcpListenerServer, TcpStreamClient, 
    SocketClient, SocketServer, 
    streaming_client::StreamingClient, streaming_server::VideoServer, {
    web_server::{WebServer, WebServerType},
    SimpleWebClient, UserBehavior,}, capture::CapFactory, 
    {dhcp_server::DhcpServer, Capture, SendMessage}, 
    dns_test_client::DnsTestClient, dns_test_server::DnsTestServer
    };
use crate::ip_generator::IpRange;
use elvis_core::{
    new_machine_arc,
    protocols::{
        arp::subnetting::{Ipv4Mask, SubnetInfo},
        dhcp::dhcp_client::DhcpClient,
        dns::{dns_client::DnsClient, dns_server::DnsServer},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        socket_api::socket::SocketType,
        tcp::Tcp,
        udp::Udp,
        Arp, Endpoint, Pci, SocketAPI,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};
use std::time::Duration;

pub async fn complex_sim() {
    let network = Network::basic();
    let user_server_ip_address: Ipv4Address = [100, 42, 0, 0].into(); // I know this one is necessary for user functionality right now
    let socket_server_ip_address: Ipv4Address = [111, 111, 11, 0].into();
    let localhost_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let client_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let server_socket_address: Endpoint = Endpoint::new(localhost_ip_address, 80);
    let server_socket_addressu: Endpoint = Endpoint::new(user_server_ip_address, 80);
    let client_socket_address: Endpoint = Endpoint::new(client_ip_address, 70);
    let dns_server_ip_address = Ipv4Address::DNS_AUTH;
    let dns_again_server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    let dhcp_server_ip_address: Ipv4Address = [123, 123, 123, 123].into(); //diff from their sim
    // look at how server_experiment generates ip_addresses 
    // This is from all the other sims initial IpTable I believe
    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();
    // From sockets don't know what this does
    let info = SubnetInfo {
        mask: Ipv4Mask::from_bitcount(0),
        default_gateway: Ipv4Address::from([1, 1, 1, 1]),
    };
    // not quite sure whether machines should be separate for each protocol/application
    // trying to be run or to whether combine them somehow
    // v1 for User_behavior
    let machines = vec![
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(user_server_ip_address)),
            WebServer::new(WebServerType::Yahoo, None),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client_ip_address)),
            UserBehavior::new(server_socket_address),
        ],
    ];
    let machines = vec![
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(localhost_ip_address)),
            TcpStreamClient::new(server_socket_address, client_socket_address),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client_ip_address)),
            TcpListenerServer::new(server_socket_address, client_socket_address),
        ],
    ];
    // Socket basic

    
    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);
    // for testing afterwards
    let mut machines_iter = machines.into_iter();
    let _server = machines_iter.next().unwrap();
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn complex_sim() {
        for _ in 0..5 {
            super::complex_sim().await;
        }
    }
}

