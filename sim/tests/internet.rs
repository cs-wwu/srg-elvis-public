use elvis::{
    applications::{Capture, SendMessage},
    core::{message::Message, Internet, Machine, Network, RcProtocol},
    protocols::{ipv4::Ipv4, tap::Tap, udp::Udp},
};

#[test]
pub fn internet() {
    let network = Network::new(vec![0, 1], 1500);

    let sender_tap = Tap::new_shared(vec![network.mtu()]);
    let sender_udp = Udp::new_shared();
    let sender_ip = Ipv4::new_shared();
    let send_message = SendMessage::new_shared("Hello!");
    let sender_protocols: [RcProtocol; 3] = [sender_udp, sender_ip, send_message];
    let sender_machine = Machine::new(sender_tap, sender_protocols.into_iter());

    let receiver_tap = Tap::new_shared(vec![network.mtu()]);
    let receiver_udp = Udp::new_shared();
    let receiver_ip = Ipv4::new_shared();
    let capture = Capture::new_shared();
    let receiver_protocols: [RcProtocol; 3] = [receiver_udp, receiver_ip, capture.clone()];
    let receiver_machine = Machine::new(receiver_tap, receiver_protocols.into_iter());

    let mut internet = Internet::new(vec![receiver_machine, sender_machine], vec![network]);
    internet.run();
    assert_eq!(
        capture.borrow().application().message().unwrap(),
        Message::new("Hello!")
    );
}
