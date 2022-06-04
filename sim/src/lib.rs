/// The core abstractions of Elvis are Message, Protocol and Session
///
/// # Message
///
/// Message is an immutable list of Bytes that are stored in one or more contiguous
/// sections of memory. Data can be pushed on or popped off a Message in very efficient
/// zero-copy operations.
///
/// # Protocol
///
/// Protocols are stackable objects that function as network processing elements.
/// A Protocol receives Messages via a `recv` method from below.
/// A Session object is created with the `open method is called`
///
/// # Session
///
/// A Session holds session state for a particular connection.
/// The `send` method on a Session sends data to successive protocol layers below.
pub mod core;

/// Utility functions
pub mod utils;
