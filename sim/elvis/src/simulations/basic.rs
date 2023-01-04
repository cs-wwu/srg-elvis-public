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

    let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    let machines = vec![
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            Pci::new_shared([network.tap()]),
            SendMessage::new_shared("Hello!", capture_ip_address, 0xbeef, None, 1),
        ]),
        Machine::new([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Pci::new_shared([network.tap()]),
            capture.clone(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn basic() {
        super::basic().await
    }
}
