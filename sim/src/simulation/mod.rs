/// Simulation specific functionality for Elvis.
/// This module currently defines the default simulation, which creates
/// a UDP sender and a UDP receiver. The sender sends one string to the
/// receiver, and the contents are checked.
use crate::{
    applications::{Capture, SendMessage},
    core::{message::Message, Internet, Machine, Network, RcProtocol},
    protocols::{ipv4::Ipv4, udp::Udp},
};

pub async fn default_simulation() {
    let mut network = Network::new(1500);

    let sender_udp = Udp::new_shared();
    let sender_ip = Ipv4::new_shared();
    let send_message = SendMessage::new_shared("Hello!");
    let sender_protocols: [RcProtocol; 3] = [sender_udp, sender_ip, send_message];
    let mut sender_machine = Machine::new(sender_protocols.into_iter(), 0);
    network.join(&mut sender_machine);

    let receiver_udp = Udp::new_shared();
    let receiver_ip = Ipv4::new_shared();
    let capture = Capture::new_shared();
    let receiver_protocols: [RcProtocol; 3] = [receiver_udp, receiver_ip, capture.clone()];
    let mut receiver_machine = Machine::new(receiver_protocols.into_iter(), 1);
    network.join(&mut receiver_machine);

    let mut internet = Internet::new(vec![receiver_machine, sender_machine], vec![network]);
    internet.run();
    assert_eq!(
        capture.borrow().application().message().unwrap(),
        Message::new("Hello!")
    );
}
