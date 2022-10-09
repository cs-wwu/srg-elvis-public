use crate::applications::PingPong;
use elvis_core::{
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
    Internet,
};

/// Runs a basic PingPong simulation.
///
/// In this simulation, two machines will send a Time To Live (TTL) message 
/// back and forth till the TTL reaches 0. TTL will be subtracted by 1 every time a machine reveives it.
pub async fn ping_pong() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));
    let ip_address_1: Ipv4Address = [123, 45, 67, 89].into();
    let ip_address_2: Ipv4Address = [123, 45, 67, 90].into();
    let ip_table: IpToNetwork = [(ip_address_1, network), (ip_address_2, network)]
        .into_iter()
        .collect();

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            PingPong::new_shared(true,  ip_address_1, ip_address_2, 0xbeef, 0xface),
        ],
        [network],
    );

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            PingPong::new_shared(false, ip_address_2, ip_address_1, 0xface, 0xbeef),
        ],
        [network],
    );

    internet.run().await;
}