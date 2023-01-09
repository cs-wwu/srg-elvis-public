use crate::applications::{Capture, Forward, SendMessage};
use elvis_core::{
    internet::NetworkHandle,
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
    Internet, Message,
};

/// Simulates a message being forwarded along across many networks.
///
/// A message is sent from one machine to another, each time being delivered
/// across a different network. When the message reaches its destination, the
/// simulation ends.
pub async fn telephone_multi() {
    let mut internet = Internet::new();
    let end = 1000;
    let networks: Vec<_> = (0..end)
        .map(|_| internet.network(Reliable::new(1500)))
        .collect();

    let remote = 0u32.to_be_bytes().into();
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(remote, networks[0])].into_iter().collect()),
            SendMessage::new_shared("Hello!", remote, 0xbeef),
        ],
        [networks[0]],
    );

    for i in 0u32..(end - 1) {
        let (local, remote, table) = create_ip_table(i, &networks);
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared(table),
                Forward::new_shared(local, remote, 0xbeef, 0xbeef),
            ],
            [networks[i as usize], networks[i as usize + 1]],
        );
    }

    let last_network = end - 1;
    let local = last_network.to_be_bytes().into();
    let last_network = last_network as usize;
    let capture = Capture::new_shared(local, 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(local, networks[last_network])].into_iter().collect()),
            capture.clone(),
        ],
        [networks[last_network]],
    );

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}

fn create_ip_table(
    network: u32,
    networks: &[NetworkHandle],
) -> (Ipv4Address, Ipv4Address, IpToNetwork) {
    let local: Ipv4Address = network.to_be_bytes().into();
    let remote: Ipv4Address = (network + 1).to_be_bytes().into();
    let network = network as usize;
    let table = [(local, networks[network]), (remote, networks[network + 1])]
        .into_iter()
        .collect();
    (local, remote, table)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn telephone_multi() {
        super::telephone_multi().await;
    }
}
