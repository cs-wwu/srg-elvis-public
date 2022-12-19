use crate::applications::QueryTester;
use elvis_core::{
    networks::Generic,
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp, Pci},
    Internet,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn query() {
    let mut internet = Internet::new();
    let mut network = Generic::new(1500);

    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared([(0.into(), 0)].into_iter().collect()),
        QueryTester::new_shared(),
        Pci::new_shared([network.tap(), network.tap()]),
    ]);

    internet.run().await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn query() {
        super::query().await
    }
}
