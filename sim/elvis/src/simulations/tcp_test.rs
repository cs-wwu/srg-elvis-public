use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Tcp,
    },
    Internet,
};

pub async fn tcp_basic() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: IpToNetwork = [(capture_ip_address, network)].into_iter().collect();

    internet.machine(
        [
            Tcp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            SendMessage::new_shared("Hello!", capture_ip_address, 0xbeef),
        ],
        [network],
    );

    let capture = Capture::new_shared(capture_ip_address, 0xbeef);
    internet.machine(
        [
            Tcp::new_shared() as SharedProtocol,
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
