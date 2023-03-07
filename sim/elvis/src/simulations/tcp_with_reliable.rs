use crate::applications::{Capture, SendMessage, Transport};
use elvis_core::{
    message::Message,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        pci::PciMonitors,
        tcp::TcpMonitors,
        Pci, Tcp,
    },
    run_internet, Machine, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_with_reliable() {
    let network = Network::basic();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();
    let pci_monitors = PciMonitors::new();
    let tcp_monitors = TcpMonitors::new();

    let message: Vec<_> = (0..20).map(|i| i as u8).collect();
    let message = Message::new(message);
    let capture = Capture::new(capture_ip_address, 0xbeef)
        .transport(Transport::Tcp)
        .shared();
    let machines = vec![
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new(tcp_monitors.clone()).shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()], pci_monitors.clone()).shared(),
            capture.clone(),
        ]),
    ];

    run_internet(
        machines,
        vec![network],
        pci_monitors
            .into_iter()
            .chain(tcp_monitors.into_iter())
            .collect(),
    )
    .await;
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_with_reliable() {
        super::tcp_with_reliable().await
    }
}
