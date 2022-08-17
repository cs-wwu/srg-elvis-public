use elvis::applications::{Forward, UnreliableTester};
use elvis_core::{
    core::{protocol::SharedProtocol, Internet},
    networks::Unreliable,
    protocols::{
        ipv4::{IpToNetwork, Ipv4Address},
        Ipv4, Udp,
    },
};

#[tokio::test]
pub async fn unreliable() {
    let mut internet = Internet::new();
    let network = internet.network(Unreliable::new(0.5));
    let tester_ip: Ipv4Address = [0, 0, 0, 0].into();
    let forward_ip: Ipv4Address = [0, 0, 0, 1].into();
    let ip_table: IpToNetwork = [(tester_ip, network), (forward_ip, network)]
        .into_iter()
        .collect();

    let tester = UnreliableTester::new_shared();
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table.clone()),
            tester.clone(),
        ],
        [network],
    );

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(ip_table),
            Forward::new_shared(forward_ip, tester_ip, 0xdead, 0xdead),
        ],
        [network],
    );

    internet.run().await;
    // We use a consistent random seed in Unreliable to decide whether a message
    // makes it. 23 happens to be the number that make it there and back with a
    // delivery probability of 50%.
    const EXPECTED_RECEIPTS: u16 = 23;
    assert_eq!(tester.application().receipt_count(), EXPECTED_RECEIPTS);
}
