use crate::applications::PingPong;
use elvis_core::{
    network::Network,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
    },
    Internet, Machine,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);

/// Runs a basic PingPong simulation.
///
/// In this simulation, two machines will send a Time To Live (TTL) message
/// back and forth till the TTL reaches 0. TTL will be subtracted by 1 every time a machine reveives it.
pub fn ping_pong() {
    let mut internet = Internet::new();
    let network = internet.add_network(Network::new());
    let ip_table: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table.clone()).shared(),
        PingPong::new(true, IP_ADDRESS_1, IP_ADDRESS_2, 0xbeef, 0xface).shared(),
    ]));
    internet.connect(machine, network);

    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(ip_table.clone()).shared(),
        PingPong::new(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef).shared(),
    ]));
    internet.connect(machine, network);

    internet.run();

    // TODO(hardint): Should check here that things actually ran correctly
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn ping_pong() {
        super::ping_pong()
    }
}
