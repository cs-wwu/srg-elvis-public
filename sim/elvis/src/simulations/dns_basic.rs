use crate::applications::{SocketClient, SocketServer};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci, Sockets,
    },
    run_internet, Machine, Network,
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
    let ip_table: Recipients = [
        (server_ip_address, Recipient::with_mac(0, 0)),
        (client1_ip_address, Recipient::with_mac(0, 1)),
        (client2_ip_address, Recipient::with_mac(0, 2)),
        (client3_ip_address, Recipient::with_mac(0, 3)),
    ]
    .into_iter()
    .collect();

    let server_socket_api = Sockets::new(Some(server_ip_address));
    let client1_socket_api = Sockets::new(Some(client1_ip_address));
    let client2_socket_api = Sockets::new(Some(client2_ip_address));
    let client3_socket_api = Sockets::new(Some(client3_ip_address));
    let machines = vec![
        // Machine::new([
        //     server_socket_api.clone(),
        //     Udp::new().shared() as SharedProtocol,
        //     Ipv4::new(ip_table.clone()).shared(),
        //     Pci::new([network.clone()]).shared(),
        //     SocketServer::new(server_socket_api, 0xbeef).shared(),
        // ]),
        // Machine::new([
        //     client1_socket_api.clone(),
        //     Udp::new().shared() as SharedProtocol,
        //     Ipv4::new(ip_table.clone()).shared(),
        //     Pci::new([network.clone()]).shared(),
        //     SocketClient::new(client1_socket_api, 1, server_ip_address, 0xbeef).shared(),
        // ]),
        // Machine::new([
        //     client2_socket_api.clone(),
        //     Udp::new().shared() as SharedProtocol,
        //     Ipv4::new(ip_table.clone()).shared(),
        //     Pci::new([network.clone()]).shared(),
        //     SocketClient::new(client2_socket_api, 2, server_ip_address, 0xbeef).shared(),
        // ]),
        // Machine::new([
        //     client3_socket_api.clone(),
        //     Udp::new().shared() as SharedProtocol,
        //     Ipv4::new(ip_table.clone()).shared(),
        //     Pci::new([network.clone()]).shared(),
        //     SocketClient::new(client3_socket_api, 3, server_ip_address, 0xbeef).shared(),
        // ]),
        new_machine![
            
        ]
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn socket_basic() {
        super::socket_basic().await;
    }
}
