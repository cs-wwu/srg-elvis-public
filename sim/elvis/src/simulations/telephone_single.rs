use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    network::Mac,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message, Network,
};

/// Simulates a message being repeatedly forwarded on a single network.
///
/// A message is passed between many machines, each attached to the same
/// network. When it reaches its destination, the simulation ends.
pub async fn telephone_single() {
    const END: u32 = 1000;
    let network = Network::basic();

    let message = Message::new("Hello!");
    let remote = 0u32.to_be_bytes().into();
    let mut machines = vec![Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new([(remote, Recipient::slot(0))].into_iter().collect()).shared(),
        Pci::new([network.tap()]).shared(),
        SendMessage::new(message.clone(), remote, 0xbeef)
            .remote_mac(1)
            .shared(),
    ])];

    for i in 0u32..(END - 1) {
        let local: Ipv4Address = i.to_be_bytes().into();
        let remote: Ipv4Address = (i + 1).to_be_bytes().into();
        let table = [(local, Recipient::slot(0)), (remote, Recipient::slot(0))]
            .into_iter()
            .collect();
        machines.push(Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(table).shared(),
            Pci::new([network.tap()]).shared(),
            Forward::new(local, remote, 0xbeef, 0xbeef, Some(i as Mac + 2)).shared(),
        ]));
    }

    let local = (END - 1).to_be_bytes().into();
    let capture = Capture::new(local, 0xbeef).shared();
    machines.push(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new([(local, Recipient::slot(0))].into_iter().collect()).shared(),
        Pci::new([network.tap()]).shared(),
        capture.clone(),
    ]));

    run_internet(machines, vec![network]).await;
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_single() {
        super::telephone_single().await
    }
}
