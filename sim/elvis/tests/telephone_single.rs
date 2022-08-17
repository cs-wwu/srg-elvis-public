use elvis_core::{
    applications::{Capture, Forward, SendMessage},
    core::{internet::NetworkHandle, protocol::SharedProtocol, Internet, Message},
    networks::Reliable,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
};

#[tokio::test]
pub async fn telephone_single() {
    let mut internet = Internet::new();
    let end = 10;
    let network = internet.network(Reliable::new(1500));

    let remote = 0u32.to_be_bytes().into();
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(remote, network)].into_iter().collect()),
            SendMessage::new_shared("Hello!", remote, 0xbeef),
        ],
        [network],
    );

    for i in 0u32..(end - 1) {
        let (local, remote, table) = create_ip_table(i, network);
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared(table),
                Forward::new_shared(local, remote, 0xbeef, 0xbeef),
            ],
            [network],
        );
    }

    let local = (end - 1).to_be_bytes().into();
    let capture = Capture::new_shared(local, 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared([(local, network)].into_iter().collect()),
            capture.clone(),
        ],
        [network],
    );

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}

fn create_ip_table(i: u32, network: NetworkHandle) -> (Ipv4Address, Ipv4Address, IpToNetwork) {
    let local: Ipv4Address = i.to_be_bytes().into();
    let remote: Ipv4Address = (i + 1).to_be_bytes().into();
    let table = [(local, network), (remote, network)].into_iter().collect();
    (local, remote, table)
}
