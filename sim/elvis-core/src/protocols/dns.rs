//! An implementation of the Domain Name Structure

pub struct Dns {

}

impl Dns {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::new(4);

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {

        }
    }
}