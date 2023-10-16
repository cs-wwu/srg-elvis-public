use std::time::Duration;

use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Endpoints, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Message, Network,
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

    let local: Ipv4Address = [127, 0, 0, 1].into();

    let mut machines = vec![new_machine![
        Udp::new(),
        Ipv4::new([(local, Recipient::with_mac(0, 1))].into_iter().collect(),),
        Pci::new([networks[0].clone()]),
        SendMessage::new(
            vec![message.clone()],
            Endpoint {
                address: remote,
                port: 0xbeef,
            },
        )
    ]];

    for i in 0u32..(END - 1) {
        let local = i.to_be_bytes().into();
        let remote = (i + 1).to_be_bytes().into();
        let table = [(local, Recipient::with_mac(1, 1))].into_iter().collect();
        machines.push(new_machine![
            Udp::new(),
            Ipv4::new(table),
            Pci::new([
                networks[i as usize].clone(),
                networks[i as usize + 1].clone(),
            ]),
            Forward::new(Endpoints::new(
                Endpoint::new(local, 0xbeef),
                Endpoint::new(remote, 0xbeef),
            ))
        ]);
    }

    let last_network = END - 1;
    let local = last_network.to_be_bytes().into();
    machines.push(new_machine![
        Udp::new(),
        Ipv4::new(IpTable::new()),
        Pci::new([networks[last_network as usize].clone()]),
        Capture::new(Endpoint::new(local, 0xbeef), 1)
    ]);

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .last()
        .unwrap()
        .into_inner()
        .protocol::<Capture>()
        .unwrap()
        .first_msg();
    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_multi() {
        super::telephone_multi().await
    }
}
