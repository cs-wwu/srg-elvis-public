use crate::applications::{SocketClient, SocketServer};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci, Sockets,
    },
    run_internet, Machine, Network,
};

/// Runs a two-way communication simulation using sockets.
///
/// In this simulation, a machine sends a "request" message to another machine.
/// The second machine receives the message, and sends back a "response" message.
/// The first machine receives that message, and sends back a "shutdown" message.
/// Finally, the second machine receives the "shutdown" message, and shuts down the simulation.
pub async fn socket_basic() {
    let network = Network::basic();
    let client_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let server_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(server_ip_address, 0), (client_ip_address, 0)]
        .into_iter()
        .collect();

    // let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    let client_socket_api = Sockets::new(Some(client_ip_address)).shared();
    let server_socket_api = Sockets::new(Some(server_ip_address)).shared();
    let machines = vec![
        Machine::new([
            client_socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SocketClient::new(
                client_socket_api,
                "Ground Control to Major Tom",
                server_ip_address,
                0xbeef,
            )
            .shared(),
        ]),
        Machine::new([
            server_socket_api.clone(),
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()]).shared(),
            SocketServer::new(server_socket_api, "Major Tom to Ground Control", 0xbeef).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn socket_basic() {
        super::socket_basic().await
    }
}
