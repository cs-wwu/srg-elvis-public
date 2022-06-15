use sim::core::Message;

#[test]
fn multi_slice() {
    let message = Message::new(b"Body")
        .with_header(b"Header")
        .slice(3, 8)
        .slice(2, 4);
    let expected = b"rB";
    assert!(message.iter().eq(expected.iter().cloned()));
}

#[test]
fn mixed_operations() {
    let message = Message::new(b"Hello, world")
        .slice(0, 5)
        .with_header(b"Header")
        .slice(3, 8);
    let expected = b"derHe";
    assert!(message.iter().eq(expected.iter().cloned()));
}
