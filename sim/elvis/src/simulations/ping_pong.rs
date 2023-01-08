use crate::applications::PingPong;
use elvis_core::{
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Tcp,
    },
    Internet,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);

/// Runs a basic PingPong simulation.
///
/// In this simulation, two machines will send a Time To Live (TTL) message
/// back and forth till the TTL reaches 0. TTL will be subtracted by 1 every time a machine reveives it.
pub async fn ping_pong() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));
    let ip_table: IpToNetwork = [(IP_ADDRESS_1, network), (IP_ADDRESS_2, network)]
        .into_iter()
        .collect();

    internet.machine(
        [
            Tcp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            PingPong::new_shared(true, IP_ADDRESS_1, IP_ADDRESS_2, 0xbeef, 0xface),
        ],
        [network],
    );

    internet.machine(
        [
            Tcp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            PingPong::new_shared(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef),
        ],
        [network],
    );

    internet.run().await;
}
