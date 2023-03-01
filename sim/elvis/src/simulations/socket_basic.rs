use crate::applications::{SocketRecvMessage, SocketSendMessage};
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
    let send_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let recv_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(recv_ip_address, 0), (send_ip_address, 0)]
        .into_iter()
        .collect();

    // let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    let send_socket_api = Sockets::new_shared(Some(send_ip_address));
    let recv_socket_api = Sockets::new_shared(Some(recv_ip_address));
    let machines = vec![
        Machine::new([
            send_socket_api.clone(),
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            Pci::new_shared([network.tap()]),
            SocketSendMessage::new_shared(
                send_socket_api.clone(),
                "Ground Control to Major Tom",
                send_ip_address,
                0xface,
                recv_ip_address,
                0xbeef,
            ),
        ]),
        Machine::new([
            recv_socket_api.clone(),
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network.tap()]),
            SocketRecvMessage::new_shared(
                recv_socket_api.clone(),
                "Major Tom to Ground Control",
                recv_ip_address,
                0xbeef,
                send_ip_address,
                0xface,
            ),
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
