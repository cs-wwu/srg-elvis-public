use crate::applications::QueryTester;
use elvis_core::{
    machine::ProtocolMapBuilder,
    network::NetworkBuilder,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
        Pci,
    },
    run_internet, Machine,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn query() {
    let network = NetworkBuilder::new().mtu(1500).build();

    let machine = Machine::new(
        ProtocolMapBuilder::new()
            .with(Udp::new())
            .with(Ipv4::new(
                [(0.into(), Recipient::with_mac(0, 0))]
                    .into_iter()
                    .collect(),
            ))
            .with(Pci::new([network.clone(), network.clone()]))
            .with(QueryTester::new().process())
            .build(),
    );

    run_internet(&[machine]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn query() {
        super::query().await;
    }
}
