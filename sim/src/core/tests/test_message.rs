#[cfg(test)]
use crate::core::{Buf, Message};

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

    let data1 = b"Hello";
    let bytes = Buf::new(data1);
    let message = message.push(&bytes);

    let data2 = b" World";
    let bytes = Buf::new(data2);
    let message = message.push(&bytes);

    assert_eq!(data1.len() + data2.len(), message.len());
}

// Test popping data off a message
#[test]
fn test_message_pop() {
    let message = Message::new();

    let data = b"Hello World";
    let bytes = Buf::new(data);
    let message = message.push(&bytes);

    let size = 3;
    let message = message.pop(size);

    assert_eq!(data.len() - size, message.len());
}

// Test retrieving the constituent chunks of a message
#[test]
fn test_message_chunks() {
    let message = Message::new();

    let data1 = b"Body";
    let bytes = Buf::new(data1);
    let message = message.push(&bytes);

    let data2 = b"Header";
    let bytes = Buf::new(data2);
    let message = message.push(&bytes);

    let chunks = message.chunks();
    assert_eq!(2, chunks.len());
    assert_eq!(data2, &chunks[0][..]);
    assert_eq!(data1, &chunks[1][..]);
}
