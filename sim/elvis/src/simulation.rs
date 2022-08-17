//! Simulation specific functionality for Elvis.
//!
//! This module currently defines
//! the default simulation, which creates a UDP sender and a UDP receiver. The
//! sender sends one string to the receiver, and the contents are checked.

use crate::applications::{Capture, SendMessage};
use elvis_core::{
    core::{message::Message, protocol::SharedProtocol, Internet},
    networks::Reliable,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn default_simulation() {
    let mut internet = Internet::new();
    let network = internet.network(Reliable::new(1500));
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
