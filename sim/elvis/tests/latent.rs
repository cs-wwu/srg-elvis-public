use elvis_core::{
    applications::{Capture, SendMessage},
    core::{protocol::SharedProtocol, Internet, Message},
    networks::Latent,
    protocols::{
        ipv4::{IpToNetwork, Ipv4Address},
        Ipv4, Udp,
    },
};
use std::time::Duration;

#[tokio::test]
pub async fn latent() {
    let mut internet = Internet::new();
    let network = internet.network(Latent::new(Duration::from_millis(250)));
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToNetwork = [(capture_ip_address, network)].into_iter().collect();

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            SendMessage::new_shared("Hello!", capture_ip_address, 0xbeef),
        ],
        [network],
    );

    let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
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
