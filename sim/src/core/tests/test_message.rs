use crate::core::Message;
use bytes::Bytes;

// Test message creation
#[test]
fn test_message_create() {
    let message = Message::new();
    assert_eq!(0, message.len());
}

// Test pushing data onto a message
#[test]
fn test_message_push() {
    let message = Message::new();

    let data1 = "Hello";
    let bytes = Bytes::from(data1);
    let message = message.push(&bytes);

    let data2 = " World";
    let bytes = Bytes::from(data2);
    let message = message.push(&bytes);

    assert_eq!(data1.len() + data2.len(), message.len());
}

// Test popping data off a message
#[test]
fn test_message_pop() {
    let message = Message::new();

    let data = "Hello World";
    let bytes = Bytes::from(data);
    let message = message.push(&bytes);

    let size = 3;
    let message = message.pop(size);

    assert_eq!(data.len() - size, message.len());
}

// Test retrieving the constituent chunks of a message
#[test]
fn test_message_chunks() {
    let message = Message::new();

    let data1 = "Body";
    let bytes = Bytes::from(data1);
    let message = message.push(&bytes);

    let data2 = "Header";
    let bytes = Bytes::from(data2);
    let message = message.push(&bytes);

    let chunks = message.chunks();
    assert_eq!(2, chunks.len());
    assert_eq!(data2, &chunks[0]);
    assert_eq!(data1, &chunks[1]);
}
