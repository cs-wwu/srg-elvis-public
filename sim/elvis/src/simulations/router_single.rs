use crate::applications::{Capture, Router, SendMessage};
use elvis_core::{
    network::Network,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
    },
    Internet, Machine, Message,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);
const IP_ADDRESS_3: Ipv4Address = Ipv4Address::new([123, 45, 67, 91]);
const IP_ADDRESS_4: Ipv4Address = Ipv4Address::new([123, 45, 67, 92]);
const DESTINATION: Ipv4Address = IP_ADDRESS_2;

// simulates a staticly configured router routing a single packet to one of three destinations
pub fn router_single() {
    let mut internet = Internet::new();
    let ip_table: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(1, 1)),
        (IP_ADDRESS_3, Recipient::with_mac(2, 1)),
        (IP_ADDRESS_4, Recipient::with_mac(3, 1)),
    ]
    .into_iter()
    .collect();

    let dt1: Recipients = [(IP_ADDRESS_2, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt2: Recipients = [(IP_ADDRESS_3, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();
    let dt3: Recipients = [(IP_ADDRESS_4, Recipient::with_mac(0, 666))]
        .into_iter()
        .collect();

    let d1 = Capture::new(IP_ADDRESS_2, 0xbeef, 1).shared();

    let networks: Vec<_> = (0..4)
        .map(|_| internet.add_network(Network::new()))
        .collect();

    // send message
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(
            [(DESTINATION, Recipient::with_mac(0, 1))]
                .into_iter()
                .collect(),
        )
        .shared(),
        SendMessage::new(vec![Message::new(b"Hello World!")], DESTINATION, 0xbeef).shared(),
    ]));
    internet.connect(machine, networks[0]);

    // machine representing our router
    let machine = internet.add_machine(Machine::new([
        Router::new(ip_table).shared() as SharedProtocol
    ]));
    internet.connect(machine, networks[0]);
    internet.connect(machine, networks[1]);
    internet.connect(machine, networks[2]);
    internet.connect(machine, networks[3]);

    // capture for destination 1
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(dt1).shared(),
        d1.clone(),
    ]));
    internet.connect(machine, networks[1]);

    // capture for destination 2
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(dt2).shared(),
    ]));
    internet.connect(machine, networks[2]);

    // capture for destination 3
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(dt3).shared(),
    ]));
    internet.connect(machine, networks[3]);

    internet.run();
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_single() {
        super::router_single()
    }
}
