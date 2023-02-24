use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::NetworkBuilder,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        Pci, Tcp,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_gigabyte_bench() {
    let network = NetworkBuilder::new().mtu(1500).build();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    let message: Vec<_> = (0..i32::MAX).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SendMessage::new(message.clone(), capture_ip_address, 0xbeef)
                .remote_mac(1)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()]).shared(),
            WaitForMessage::new(capture_ip_address, 0xbeef, message)
                .transport(Transport::Tcp)
                .disable_checking()
                .shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}
