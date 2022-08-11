use elvis::{
    applications::{Capture, Forward, SendMessage},
    core::{Internet, Message, NetworkId, SharedProtocol},
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
};

#[tokio::test]
pub async fn telephone() {
    let mut internet = Internet::new();
    let end = 10;
    for _ in 0..end {
        internet.network(1500);
    }

    let remote = 0u32.to_be_bytes().into();
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(remote, 0)].into_iter().collect()),
            SendMessage::new_shared("Hello!", Ipv4Address::LOCALHOST, remote, 0xbeef, 0xbeef),
        ],
        [0],
    );

    for i in 0u32..(end - 1) {
        let (local, remote, table) = create_ip_table(i);
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared(table),
                Forward::new_shared(local, remote, 0xbeef, 0xbeef),
            ],
            [i, i + 1],
        );
    }

    let last_network = end - 1;
    let local = last_network.to_be_bytes().into();
    let capture = Capture::new_shared(local, 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(local, last_network)].into_iter().collect()),
            capture.clone(),
        ],
        [last_network],
    );

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}

fn create_ip_table(network: NetworkId) -> (Ipv4Address, Ipv4Address, IpToNetwork) {
    let local: Ipv4Address = network.to_be_bytes().into();
    let remote: Ipv4Address = (network + 1).to_be_bytes().into();
    let table = [(local, network), (remote, network + 1)]
        .into_iter()
        .collect();
    (local, remote, table)
}
