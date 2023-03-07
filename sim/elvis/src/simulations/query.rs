use crate::applications::QueryTester;
use elvis_core::{
    network::NetworkBuilder,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Recipient},
        pci::PciMonitors,
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
    let pci_monitors = PciMonitors::new();

    let machine = Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new([(0.into(), Recipient::new(0, 0))].into_iter().collect()).shared(),
        QueryTester::new().shared(),
        Pci::new([network.tap(), network.tap()], pci_monitors.clone()).shared(),
    ]);

    run_internet(
        vec![machine],
        vec![network],
        pci_monitors.into_iter().collect(),
    )
    .await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn query() {
        super::query().await;
    }
}
