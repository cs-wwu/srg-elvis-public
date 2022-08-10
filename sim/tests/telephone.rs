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

    let (local, remote, table) = create_ip_table(0);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(table),
            SendMessage::new_shared("Hello!", local, remote, 0xbeef, 0xbeef),
        ],
        [0, 1],
    );

    for i in 1u32..(end - 1) {
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

    let capture = Capture::new_shared(end.to_be_bytes().into(), 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(
                [((end - 1).to_be_bytes().into(), end - 1)]
                    .into_iter()
                    .collect(),
            ),
            capture.clone(),
        ],
        [end - 1],
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
