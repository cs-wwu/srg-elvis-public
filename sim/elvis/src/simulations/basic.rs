use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient, Recipients},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
    let network = Network::basic();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::new(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new("Hello!");
    let capture = Capture::new(capture_ip_address, 0xbeef).shared();
    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.clone()]).shared(),
            SendMessage::new(vec![message.clone()], capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table).shared(),
            Pci::new([network.clone()]).shared(),
            capture.clone(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
    assert_eq!(capture.application().message(), Some(message),);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn basic() {
        super::basic().await
    }
}
