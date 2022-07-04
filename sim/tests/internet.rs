use elvis::{
    applications::{Capture, SendMessage},
    core::{Internet, InternetError, Machine, Message, Network, RcProtocol},
    protocols::{Ipv4, Tap},
};

// Todo: Test that the message is actually received
// Todo: Test both send and receive

#[test]
pub fn internet() -> Result<(), InternetError> {
    let network = Network::new(vec![0, 1], 1500);

    let sender_tap = Tap::new_shared(vec![network.mtu()]);
    let sender_ip = Ipv4::new_shared();
    let send_message = SendMessage::new_shared("Hello!");
    let sender_protocols: [RcProtocol; 2] = [sender_ip, send_message];
    let sender_machine = Machine::new(sender_tap, sender_protocols.into_iter())?;

    let receiver_tap = Tap::new_shared(vec![network.mtu()]);
    let receiver_ip = Ipv4::new_shared();
    let capture = Capture::new_shared();
    let receiver_protocols: [RcProtocol; 2] = [receiver_ip, capture.clone()];
    let receiver_machine = Machine::new(receiver_tap, receiver_protocols.into_iter())?;

    let mut internet = Internet::new(vec![receiver_machine, sender_machine], vec![network]);
    internet.run()?;
    assert_eq!(
        capture.borrow().application().message().unwrap(),
        Message::new("Hello!")
    );

    Ok(())
}
