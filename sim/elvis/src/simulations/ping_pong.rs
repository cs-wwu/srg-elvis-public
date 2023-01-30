use crate::applications::PingPong;
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4, Ipv4Address},
        udp::Udp,
        Pci,
    },
    run_internet, Machine, Network,
};

const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);

/// Runs a basic PingPong simulation.
///
/// In this simulation, two machines will send a Time To Live (TTL) message
/// back and forth till the TTL reaches 0. TTL will be subtracted by 1 every time a machine reveives it.
pub async fn ping_pong() {
    let network = Network::basic();
    let ip_table: IpToTapSlot = [(IP_ADDRESS_1, 0), (IP_ADDRESS_2, 0)].into_iter().collect();

    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new_shared([network.tap()]),
            PingPong::new(true, IP_ADDRESS_1, IP_ADDRESS_2, 0xbeef, 0xface).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new_shared([network.tap()]),
            PingPong::new(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;

    // TODO(hardint): Should check here that things actually ran correctly
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn ping_pong() {
        super::ping_pong().await
    }
}
