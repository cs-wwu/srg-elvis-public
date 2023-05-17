use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    network::Mac,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci, UserProcess,
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
    let mut machines = vec![Machine::new(
        ProtocolMapBuilder::new()
            .with(Udp::new())
            .with(Ipv4::new(
                [(remote, Recipient::with_mac(0, 1))].into_iter().collect(),
            ))
            .with(Pci::new([network.clone()]))
            .with(SendMessage::new(vec![message.clone()], Endpoint::new(remote, 0xbeef)).process())
            .build(),
    )];

    for i in 0u32..(END - 1) {
        let local: Ipv4Address = i.to_be_bytes().into();
        let remote: Ipv4Address = (i + 1).to_be_bytes().into();
        let table = [(remote, Recipient::with_mac(0, i as Mac + 2))]
            .into_iter()
            .collect();
        machines.push(Machine::new(
            ProtocolMapBuilder::new()
                .with(Udp::new())
                .with(Ipv4::new(table))
                .with(Pci::new([network.clone()]))
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

    let local = (END - 1).to_be_bytes().into();
    machines.push(Machine::new(
        ProtocolMapBuilder::new()
            .with(Udp::new())
            .with(Ipv4::new(Default::default()))
            .with(Pci::new([network.clone()]))
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
    async fn telephone_single() {
        super::telephone_single().await
    }
}
