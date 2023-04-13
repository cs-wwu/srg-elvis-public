use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    network::{Mac, Network},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
    },
    Internet, Machine, Message,
};

const END: u32 = 1000;

/// Simulates a message being repeatedly forwarded on a single network.
///
/// A message is passed between many machines, each attached to the same
/// network. When it reaches its destination, the simulation ends.
pub fn telephone_single() {
    let mut internet = Internet::new();
    let network = internet.add_network(Network::new());

    let message = Message::new("Hello!");
    let remote = 0u32.to_be_bytes().into();
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new([(remote, Recipient::with_mac(0, 1))].into_iter().collect()).shared(),
        SendMessage::new(vec![message.clone()], remote, 0xbeef).shared(),
    ]));
    internet.connect(machine, network);

    for i in 0u32..(END - 1) {
        let local: Ipv4Address = i.to_be_bytes().into();
        let remote: Ipv4Address = (i + 1).to_be_bytes().into();
        let table = [(remote, Recipient::with_mac(0, i as Mac + 2))]
            .into_iter()
            .collect();
        let machine = internet.add_machine(Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(table).shared(),
            Forward::new(local, remote, 0xbeef, 0xbeef).shared(),
        ]));
        internet.connect(machine, network);
    }

    let local = (END - 1).to_be_bytes().into();
    let capture = Capture::new(local, 0xbeef, 1).shared();
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(Default::default()).shared(),
        capture.clone(),
    ]));
    internet.connect(machine, network);

    internet.run();
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[test]
    fn telephone_single() {
        super::telephone_single()
    }
}
