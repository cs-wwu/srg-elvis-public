use crate::applications::{Capture, SendMessage, Transport};
use elvis_core::{
    message::Message,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
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

    let message: Vec<_> = (0..20).map(|i| i as u8).collect();
    let message = Message::new(message);
    let capture = Capture::new(capture_ip_address, 0xbeef)
        .transport(Transport::Tcp)
        .shared();
    let machines = vec![
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            SendMessage::new(message.clone(), capture_ip_address, 0xbeef)
                .transport(Transport::Tcp)
                .shared(),
        ]),
        Machine::new([
            Tcp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.tap()]).shared(),
            capture.clone(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tcp_with_reliable() {
        super::tcp_with_reliable().await
    }
}
