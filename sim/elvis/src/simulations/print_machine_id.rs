use crate::applications::PrintMachineId;
use elvis_core::{
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4, Ipv4Address},
        udp::Udp,
    },
    Internet,
};

/// A simulation that prints the ID of each machine in the simulation.
pub async fn print_machine_id() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));

    for _ in 0..3 {
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared([(Ipv4Address::LOCALHOST, network)].into_iter().collect()),
                PrintMachineId::new_shared(),
            ],
            [network],
        )
    }

    internet.run().await;
}
