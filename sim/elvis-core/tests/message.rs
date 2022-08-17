use elvis_core::message::Message;

#[test]
fn multi_slice() {
    let mut message = Message::new(b"Body");
    message.prepend(b"Header");
    message.slice(3..8);
    message.slice(2..4);
    let expected = b"rB";
    assert!(message.iter().eq(expected.iter().cloned()));
}

#[test]
fn mixed_operations() {
    let mut message = Message::new(b"Hello, world");
    message.slice(0..5);
    message.prepend(b"Header");
    message.slice(3..8);
    let expected = b"derHe";
    assert!(message.iter().eq(expected.iter().cloned()));
}

#[test]
fn sliced_chunk() {
    let mut message = Message::new(b"Hello, world");
    message.slice(7..);
    message.prepend(b"Header ");
    let expected = b"Header world";
    assert!(message.iter().eq(expected.iter().cloned()));
}
