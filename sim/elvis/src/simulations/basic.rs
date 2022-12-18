use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    networks::Generic,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
    },
    Internet,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
    let mut internet = Internet::new();
    let mut network = Generic::new(1500);
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToTapSlot = [(capture_ip_address, 0)].into_iter().collect();

    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared(ip_table.clone()),
        Pci::new_shared([network.tap()]),
        SendMessage::new_shared("Hello!", capture_ip_address, 0xbeef, None),
    ]);

    let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared(ip_table),
        Pci::new_shared([network.tap()]),
        capture.clone(),
    ]);

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}
