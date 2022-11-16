use crate::{applications::{Capture, SendMessage}, parsing::generate_sim};
use elvis_core::{
    message::Message,
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToNetwork, Ipv4, Ipv4Address},
        udp::Udp,
    },
    Internet,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
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

    let s: &str = "[Machine name='test' net-id='1' net-id2='4' net-id3='2'][Machine name='test' net-id='3' net-id2='2']";
    generate_sim(s);

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}
