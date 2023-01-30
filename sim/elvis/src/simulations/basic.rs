use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
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
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    let message = Message::new("Hello!");
    let capture = Capture::new(capture_ip_address, 0xbeef).shared();
    let machines = vec![
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            Pci::new_shared([network.tap()]),
            SendMessage::new(message.clone(), capture_ip_address, 0xbeef).shared(),
        ]),
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network.tap()]),
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
