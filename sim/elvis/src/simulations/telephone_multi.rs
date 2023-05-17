use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci, UserProcess,
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
            .with(Udp::new())
            .with(Ipv4::new(
                [(remote, Recipient::with_mac(0, 1))].into_iter().collect(),
            ))
            .with(Pci::new([networks[0].clone()]))
            .with(
                SendMessage::new(
                    vec![message.clone()],
                    Endpoint {
                        address: remote,
                        port: 0xbeef,
                    },
                )
                .process(),
            )
            .build(),
    )];

    for i in 0u32..(END - 1) {
        let local = i.to_be_bytes().into();
        let remote = (i + 1).to_be_bytes().into();
        let table = [(remote, Recipient::with_mac(1, 1))].into_iter().collect();
        machines.push(Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(table))
                .with(Pci::new([
                    networks[i as usize].clone(),
                    networks[i as usize + 1].clone(),
                ]))
                .with(
                    Forward::new(Endpoints::new(
                        Endpoint::new(local, 0xbeef),
                        Endpoint::new(remote, 0xbeef),
                    ))
                    .process(),
                )
                .build(),
        ));
    }

    let last_network = END - 1;
    let local = last_network.to_be_bytes().into();
    machines.push(Machine::new(
        ProtocolMapBuilder::new()
            .with(Udp::new())
            .with(Ipv4::new(Default::default()))
            .with(Pci::new([networks[last_network as usize].clone()]))
            .with(Capture::new(Endpoint::new(local, 0xbeef), 1).process())
            .build(),
    ));

    run_internet(&machines).await;
    let received = machines
        .into_iter()
        .last()
        .unwrap()
        .into_inner()
        .protocol::<UserProcess<Capture>>()
        .unwrap()
        .application()
        .message();
    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_multi() {
        super::telephone_multi().await
    }
}
