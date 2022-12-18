use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    networks::Generic,
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp, Pci},
    Internet, Message,
};

/// Simulates a message being forwarded along across many networks.
///
/// A message is sent from one machine to another, each time being delivered
/// across a different network. When the message reaches its destination, the
/// simulation ends.
pub async fn telephone_multi() {
    let mut internet = Internet::new();
    const END: u32 = 1000;
    // Since we are using a broadcast network, the destination MAC is not used
    let mut networks: Vec<_> = (0..END).map(|_| Generic::new_opaque()).collect();

    let remote = 0u32.to_be_bytes().into();
    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared([(remote, 0)].into_iter().collect()),
        Pci::new_shared([networks[0].tap()]),
        SendMessage::new_shared("Hello!", remote, 0xbeef, None),
    ]);

    for i in 0u32..(END - 1) {
        let local = i.to_be_bytes().into();
        let remote = (i + 1).to_be_bytes().into();
        let table = [(local, 0), (remote, 1)].into_iter().collect();
        internet.machine([
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(table),
            Forward::new_shared(local, remote, 0xbeef, 0xbeef, None),
            Pci::new_shared([networks[i as usize].tap(), networks[i as usize + 1].tap()]),
        ]);
    }

    let last_network = END - 1;
    let local = last_network.to_be_bytes().into();
    let capture = Capture::new_shared(local, 0xbeef);
    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared([(local, last_network)].into_iter().collect()),
        Pci::new_shared([networks[last_network as usize].tap()]),
        capture.clone(),
    ]);

    internet.run(networks).await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}
