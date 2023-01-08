use crate::{
    applications::{Capture, SendMessage},
    networks::Latent,
};
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4Address},
        Ipv4, Tcp,
    },
    Internet, Message,
};
use std::time::Duration;

/// Runs a basic simulation using a network with latency.
///
/// This simulation is identical to [`basic`](super::basic()) except that it uses
/// a [`Latent`] network instead of a
/// [`Reliable`](elvis_core::networks::Reliable) one.
pub async fn latent() {
    let mut internet = Internet::new();
    let network = internet.network(Latent::new(Duration::from_millis(250)));
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
