use crate::applications::{SocketPingClient, SocketPongServer};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
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
pub async fn socket_ping_pong() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let client_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let ip_table: IpToTapSlot = [
        (server_ip_address, 0),
        (client_ip_address, 0),
    ]
    .into_iter()
    .collect();

    let server_socket_api = Sockets::new(Some(server_ip_address)).shared();
    let client_socket_api = Sockets::new(Some(client_ip_address)).shared();
    let machines = vec![
        Machine::new([
            server_socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SocketPongServer::new(server_socket_api, 0xbeef).shared(),
        ]),
        Machine::new([
            client_socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SocketPingClient::new(client_socket_api, 1, server_ip_address, 0xbeef).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn socket_ping_pong() {
        super::socket_ping_pong().await;
    }
}
