// Import submodules
mod message;
mod protocol;

// Export types at the same level as core, so we get core::Message
pub use message::*;
pub use protocol::*;
