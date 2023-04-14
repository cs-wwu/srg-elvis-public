use crate::applications::{Capture, SendMessage, Transport};
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
pub fn tcp_with_reliable() {
    let mut internet = Internet::new();
    let network = internet.add_network(Network::new());
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..20).map(|i| i as u8).collect();
    let message = Message::new(message);
    let capture = Capture::new(capture_ip_address, 0xbeef, 1)
        .transport(Transport::Tcp)
        .shared();

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
        capture.clone(),
    ]));
    internet.connect(machine, network);

    internet.run();
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[test]
    fn tcp_with_reliable() {
        super::tcp_with_reliable()
    }
}
