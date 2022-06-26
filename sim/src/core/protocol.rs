use std::collections::HashSet;

use crate::core::Message;

pub type ProtocolId = u32;

/// Protocols are stackable objects that function as network processing
/// elements. Protocols have Protocols stacked above them and Protocols stacked
/// below them. `set_up` and `set_down` are used to create the stacking order.
///
/// Invoke `open` on a Protocol to create a Session object.
/// A Protocol maintains a list of Session objects that encapsulate connection
/// state.
///
/// Protcols expose methods to send and receive Messages.
///
/// # Receive Path
///
/// A Protocol receives Messages via a `recv` method from below.
/// The Message header is examined to determine the appropriate Session object.
/// The session's `recv` method is called to route the message appropriately.
/// The Session object may strip headers, and then call `recv` on a higher level
/// Protocol.
///
/// # Send Path
///
/// A Session is invoked with a `send` method from above.
/// The Session may add headers, and send the message onward to the Protocol
/// object below. The Protocol object is expected to demux the message to the
/// right Session, and invoke the Sessions's `send` method.
pub trait Protocol {
    /// Return an identifier for the protocol. Identifiers are 32 bit constants
    /// statically assigned throughout the simulation. This simplifies
    /// Protocols/Sessions demultiplexing to the right protocol on message
    /// receipt
    fn id(&self) -> ProtocolId;

    fn open_active(
        &mut self,
        invoker: &dyn Protocol,
        participants: ParticipantSet,
    ) -> Box<dyn Session>;

    fn open_passive(
        &mut self,
        invoker: &dyn Protocol,
        participants: ParticipantSet,
    ) -> Box<dyn Session>;

    fn add_capability(&mut self, invoker: &dyn Protocol, participants: ParticipantSet);

    fn demux(&self, message: Message) -> dyn Session;
}

/// A Session holds state for a particular connection. A Session "belongs"
/// to a Protocol.
pub trait Session {
    /// Return the Protocol that this Session belongs to
    fn protocol(&self) -> dyn Protocol;

    /// Invoked from a Protocol to send a Message.
    ///
    /// # Arguments
    ///
    /// * `message` - The Message to send.
    fn send(&self, message: Message) -> Result<(), Box<dyn std::error::Error>>;

    /// Invoked from a Protocol or Session object below for Message receipt.
    ///
    /// # Arguments
    ///
    /// * `message` - The Message to receive.
    fn recv(&self, message: Message) -> Result<(), Box<dyn std::error::Error>>;
}

pub type ParticipantSet = HashSet<(&'static str, Primitive)>;

pub enum Primitive {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
}
