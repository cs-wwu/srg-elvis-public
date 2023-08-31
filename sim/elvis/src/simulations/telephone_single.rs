use std::time::Duration;

use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    network::Mac,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Message, Network,
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

    let local: Ipv4Address = [127, 0, 0, 1].into();

    let mut machines = vec![new_machine![
        Udp::new(),
        Ipv4::new([(local, Recipient::with_mac(0, 1))].into_iter().collect(),),
        Pci::new([network.clone()]),
        SendMessage::new(vec![message.clone()], Endpoint::new(remote, 0xbeef))
    ]];

    for i in 0u32..(END - 1) {
        let local: Ipv4Address = i.to_be_bytes().into();
        let remote: Ipv4Address = (i + 1).to_be_bytes().into();
        let table = [(local, Recipient::with_mac(0, i as Mac + 2))]
            .into_iter()
            .collect();
        machines.push(new_machine![
            Udp::new(),
            Ipv4::new(table),
            Pci::new([network.clone()]),
            Forward::new(Endpoints::new(
                Endpoint::new(local, 0xbeef),
                Endpoint::new(remote, 0xbeef),
            ))
        ]);
    }

    let local = (END - 1).to_be_bytes().into();
    machines.push(new_machine![
        Udp::new(),
        Ipv4::new(IpTable::new()),
        Pci::new([network.clone()]),
        Capture::new(Endpoint::new(local, 0xbeef), 1)
    ]);

    let status = run_internet_with_timeout(&machines, Duration::from_secs(5)).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .last()
        .unwrap()
        .into_inner()
        .protocol::<Capture>()
        .unwrap()
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
