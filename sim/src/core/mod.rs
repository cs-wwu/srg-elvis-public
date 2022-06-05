// Import submodules
mod buf;
mod message;
mod protocol;

// Export types at the same level as core, so we get core::Message
pub use self::buf::Buf;
pub use self::message::Message;
pub use self::protocol::Protocol;
pub use self::protocol::Session;

// Import tests
mod tests;
