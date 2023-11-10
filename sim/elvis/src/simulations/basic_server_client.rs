use std::time::Duration;

use crate::applications::{BasicClient, BasicServer};
use elvis_core::{
    new_machine_arc,
    protocols::{
        arp::subnetting::{Ipv4Mask, SubnetInfo},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        tcp::Tcp,
        udp::Udp,
        Arp, Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network, Transport,
};

/// Runs a basic server-client simulation using sockets.
///
/// In this simulation, several client machines send "request" messages to a
/// server machine. The server receives the requests, and sends back
/// "response" messages to each client. The clients receive those
/// responses, and each send back an "ackowledgement" message. The server
/// receives the "ackowledgement" messages, and shuts down the simulation.
pub async fn basic_server_client(
    transport: Transport,
    num_clients: u8,
    output: bool,
    delay_ms: u16,
) -> ExitStatus {
    let network = Network::basic();
    let server_ip = Ipv4Address::new([111, 111, 11, 0]);
    let server_endpoint = Endpoint::new(server_ip, 0xbeef); //Ipv4Address = [111, 111, 11, 0].into();

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let info = SubnetInfo {
        mask: Ipv4Mask::from_bitcount(0),
        default_gateway: Ipv4Address::from([1, 1, 1, 1]),
    };

    let mut machines = vec![new_machine_arc![
        Udp::new(),
        Tcp::new(),
        Ipv4::new(ip_table.clone()),
        Pci::new([network.clone()]),
        Arp::new().preconfig_subnet(server_ip, info),
        BasicServer::new(server_endpoint, transport, output, num_clients)
    ]];
    for i in 1..=num_clients {
        machines.push(new_machine_arc![
            Udp::new(),
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new().preconfig_subnet([111, 111, 11, i].into(), info),
            BasicClient::new(
                i.into(),
                server_endpoint,
                [111, 111, 11, i].into(),
                transport,
                output,
                delay_ms
            ),
        ])
    }

    let status = run_internet_with_timeout(
        &machines,
        Duration::from_millis((u64::from(delay_ms) * u64::from(num_clients) * 2) + 1000),
    )
    .await;
    assert_eq!(status, ExitStatus::Exited);
    status
}
