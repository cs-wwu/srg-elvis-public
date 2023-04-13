use crate::applications::QueryTester;
use elvis_core::{
    network::Network,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
    },
    Internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub fn query() {
    let mut internet = Internet::new();
    let networks: Vec<_> = (0..2)
        .map(|_| internet.add_network(Network::new().mtu(1500)))
        .collect();
    let machine = internet.add_machine(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(
            [(0.into(), Recipient::with_mac(0, 0))]
                .into_iter()
                .collect(),
        )
        .shared(),
        QueryTester::new().shared(),
    ]));
    for network in networks {
        internet.connect(machine, network);
    }
    internet.run();
}

#[cfg(test)]
mod tests {
    #[test]
    fn query() {
        super::query();
    }
}
