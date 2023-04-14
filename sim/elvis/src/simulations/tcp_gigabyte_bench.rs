use crate::applications::{SendMessage, Transport, WaitForMessage};
use elvis_core::{
    message::Message,
    network::Network,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        Tcp,
    },
    Internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub fn tcp_gigabyte_bench() {
    let mut internet = Internet::new();
    let network = internet.add_network(Network::new().mtu(1500));
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..1_000_000_000).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machine = internet.add_machine(Machine::new([
        Tcp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table.clone()).shared(),
        SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef)
            .transport(Transport::Tcp)
            .shared(),
    ]));
    internet.connect(machine, network);

    let machine = internet.add_machine(Machine::new([
        Tcp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table).shared(),
        WaitForMessage::new(capture_ip_address, 0xbeef, message)
            .transport(Transport::Tcp)
            .disable_checking()
            .shared(),
    ]));
    internet.connect(machine, network);

    internet.run();
}
