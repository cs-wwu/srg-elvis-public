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
