use elvis::{
    applications::{Capture, SendMessage},
    core::{Internet, InternetError, Machine, Message, Network, RcProtocol},
    protocols::Tap,
};
use std::iter;

// Todo: Test that the message is actually received
// Todo: Test both send and receive

#[test]
pub fn internet() -> Result<(), InternetError> {
    let network = Network::new(vec![0, 1], 1500);

    let sender_tap = Tap::new_shared(vec![network.mtu()]);
    let send_message = SendMessage::new_shared("Hello!");
    let sender_machine = Machine::new(sender_tap, iter::once(send_message as RcProtocol))?;

    let receiver_tap = Tap::new_shared(vec![network.mtu()]);
    let capture = Capture::new_shared();
    let receiver_machine = Machine::new(receiver_tap, iter::once(capture.clone() as RcProtocol))?;

    let mut internet = Internet::new(vec![receiver_machine, sender_machine], vec![network]);
    internet.run()?;
    assert_eq!(
        capture.borrow().application().message().unwrap(),
        Message::new("Hello!")
    );

    Ok(())
}
