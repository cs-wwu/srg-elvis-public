/// Simulation specific functionality for Elvis. This module currently defines
/// the default simulation, which creates a UDP sender and a UDP receiver. The
/// sender sends one string to the receiver, and the contents are checked.
use crate::{
    applications::{Capture, SendMessage},
    core::{message::Message, Internet, SharedProtocol},
    protocols::{
        ipv4::{Ipv4, Ipv4Address},
        udp::Udp,
    },
};

pub async fn default_simulation() {
    let mut internet = Internet::new();
    let network = internet.network(1500);

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(),
            SendMessage::new_shared(
                "Hello!",
                Ipv4Address::LOCALHOST,
                [123, 45, 67, 89].into(),
                0xdead,
                0xbeef,
            ),
        ],
        [network],
    );

    let capture = Capture::new_shared([123, 45, 67, 89].into(), 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(),
            capture.clone(),
        ],
        [network],
    );

    internet.run().await;
    assert_eq!(
        capture.lock().unwrap().application().message().unwrap(),
        Message::new("Hello!")
    );
}
