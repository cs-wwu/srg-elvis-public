use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
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
    let mut machines = vec![Machine::new(
        ProtocolMapBuilder::new()
            .udp(Udp::new())
            .ipv4(Ipv4::new(
                [(remote, Recipient::with_mac(0, 1))].into_iter().collect(),
            ))
            .pci(Pci::new([networks[0].clone()]))
            .other(SendMessage::new(vec![message.clone()], remote, 0xbeef).shared())
            .build(),
    )];

    for i in 0u32..(END - 1) {
        let local = i.to_be_bytes().into();
        let remote = (i + 1).to_be_bytes().into();
        let table = [(remote, Recipient::with_mac(1, 1))].into_iter().collect();
        machines.push(Machine::new(
            ProtocolMapBuilder::new()
                .udp(Udp::new())
                .ipv4(Ipv4::new(table))
                .pci(Pci::new([
                    networks[i as usize].clone(),
                    networks[i as usize + 1].clone(),
                ]))
                .other(Forward::new(local, remote, 0xbeef, 0xbeef).shared())
                .build(),
        ));
    }

    let last_network = END - 1;
    let local = last_network.to_be_bytes().into();
    let capture = Capture::new(local, 0xbeef, 1).shared();
    machines.push(Machine::new(
        ProtocolMapBuilder::new()
            .udp(Udp::new())
            .ipv4(Ipv4::new(Default::default()))
            .pci(Pci::new([networks[last_network as usize].clone()]))
            .other(capture.clone())
            .build(),
    ));

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
