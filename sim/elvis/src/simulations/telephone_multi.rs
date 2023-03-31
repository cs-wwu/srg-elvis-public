use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Message, Network,
};

/// Simulates a message being forwarded along across many networks.
///
/// A message is sent from one machine to another, each time being delivered
/// across a different network. When the message reaches its destination, the
/// simulation ends.
pub async fn telephone_multi() {
    const END: u32 = 1000;
    // Since we are using a broadcast network, the destination MAC is not used
    let networks: Vec<_> = (0..END).map(|_| Network::basic()).collect();

    let message = Message::new("Hello!");
    let remote = 0u32.to_be_bytes().into();
    let mut machines = vec![Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new([(remote, Recipient::new(0, 1))].into_iter().collect()).shared(),
        Pci::new([networks[0].clone()]).shared(),
        SendMessage::new(vec![message.clone()], remote, 0xbeef).shared(),
    ])];

    for i in 0u32..(END - 1) {
        let local = i.to_be_bytes().into();
        let remote = (i + 1).to_be_bytes().into();
        let table = [(remote, Recipient::new(1, 1))].into_iter().collect();
        machines.push(Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(table).shared(),
            Forward::new(local, remote, 0xbeef, 0xbeef).shared(),
            Pci::new([
                networks[i as usize].clone(),
                networks[i as usize + 1].clone(),
            ])
            .shared(),
        ]));
    }

    let last_network = END - 1;
    let local = last_network.to_be_bytes().into();
    let capture = Capture::new(local, 0xbeef).shared();
    machines.push(Machine::new([
        Udp::new().shared() as SharedProtocol,
        Ipv4::new(Default::default()).shared(),
        Pci::new([networks[last_network as usize].clone()]).shared(),
        capture.clone(),
    ]));

    run_internet(machines, networks).await;
    assert_eq!(capture.application().message(), Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_multi() {
        super::telephone_multi().await
    }
}
