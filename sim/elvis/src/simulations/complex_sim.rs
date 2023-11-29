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
    let dns_server_ip_address = Ipv4Address::DNS_AUTH;
    let dns_again_server_ip_address: Ipv4Address = [123, 45, 67, 15].into();
    let dhcp_server_ip_address: Ipv4Address = [123, 123, 123, 123].into(); //diff from their sim
    let localhost_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    // look at how server_experiment generates ip_addresses 
}