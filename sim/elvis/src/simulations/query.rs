use std::collections::HashSet;

use crate::applications::Query;
use elvis_core::{
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address},
        udp::Udp,
    },
    Internet,
};
use tokio::sync::mpsc;

/// A simulation that prints the ID of each machine in the simulation.
pub async fn query() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));
    const MACHINE_COUNT: usize = 3;

    let (tx, mut rx) = mpsc::channel(MACHINE_COUNT);

    for _ in 0..MACHINE_COUNT {
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared([(Ipv4Address::LOCALHOST, network)].into_iter().collect()),
                Query::new_shared(tx.clone()),
            ],
            [network],
        )
    }

    internet.run().await;

    let mut ids = HashSet::new();
    for _ in 0..MACHINE_COUNT {
        let id = rx.recv().await;
        ids.insert(id);
    }

    for i in 0..MACHINE_COUNT as u64 {
        assert!(ids.contains(&Some(i)));
    }
}
