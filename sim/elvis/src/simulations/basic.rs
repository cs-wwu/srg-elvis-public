use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    network::Network,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
    },
    Internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub fn basic() {
    let mut internet = Internet::new();

    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();
    let message = Message::new("Hello!");
    let capture = Capture::new(capture_ip_address, 0xbeef, 1).shared();

    let machine_1 = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table.clone()).shared(),
        SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef).shared(),
    ]));

    let machine_2 = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table).shared(),
        capture.clone(),
    ]));

    let network = internet.add_network(Network::new());

    internet.connect(machine_1, network);
    internet.connect(machine_2, network);

    internet.run();
    assert_eq!(capture.application().message(), Some(message),);
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        super::basic()
    }
}
