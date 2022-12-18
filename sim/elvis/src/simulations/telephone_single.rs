use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    networks::{Generic, Mac},
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
    },
    Internet, Message,
};

/// Simulates a message being repeatedly forwarded on a single network.
///
/// A message is passed between many machines, each attached to the same
/// network. When it reaches its destination, the simulation ends.
pub async fn telephone_single() {
    let mut internet = Internet::new();
    const END: u32 = 1000;
    let mut network = Generic::new_opaque();

    let remote = 0u32.to_be_bytes().into();
    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared([(remote, 0)].into_iter().collect()),
        Pci::new_shared([network.tap()]),
        SendMessage::new_shared("Hello!", remote, 0xbeef, Some(1)),
    ]);

    for i in 0u32..(END - 1) {
        let local: Ipv4Address = i.to_be_bytes().into();
        let remote: Ipv4Address = (i + 1).to_be_bytes().into();
        let table = [(local, 0), (remote, 0)].into_iter().collect();
        internet.machine([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(table),
            Pci::new_shared([network.tap()]),
            Forward::new_shared(local, remote, 0xbeef, 0xbeef, Some(i as Mac + 2)),
        ]);
    }

    let local = (END - 1).to_be_bytes().into();
    let capture = Capture::new_shared(local, 0xbeef);
    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared([(local, 0)].into_iter().collect()),
        Pci::new_shared([network.tap()]),
        capture.clone(),
    ]);

    internet.run([network]).await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}
