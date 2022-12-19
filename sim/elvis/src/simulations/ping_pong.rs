use crate::applications::PingPong;
use elvis_core::{
    networks::Generic,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
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
    let mut network = Generic::new(1500);
    let ip_table: IpToTapSlot = [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 0)].into_iter().collect();

    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared(ip_table.clone()),
        Pci::new_shared([network.tap()]),
        PingPong::new_shared(true, IP_ADDRESS_1, IP_ADDRESS_2, 0xbeef, 0xface),
    ]);

    internet.machine([
        Udp::new_shared() as SharedProtocol,
        Ipv4::new_shared(ip_table.clone()),
        Pci::new_shared([network.tap()]),
        PingPong::new_shared(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef),
    ]);

    internet.run().await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn ping_pong() {
        super::ping_pong().await
    }
}
