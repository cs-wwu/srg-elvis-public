use crate::core::Message;
use std::{collections::HashSet, error::Error};

pub type ProtocolId = u32;
pub type SessionId = u32;

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
    /// Returns a unique identifier for the protocol.
    fn id(&self) -> ProtocolId;

    /// Called by a high-level protocol to open a new session.
    ///
    /// # Arguments
    ///
    /// * `invoker` is the higher-level protocol that requested to open the
    ///   session.
    /// * `participants` is an identifier for the session. For example, one
    ///   might open a session on a UDP protocol with the participant set
    ///   {source_address, source_port, destination_address, destination_port}.
    ///   The UDP protocol would then save a mapping of this participant set to
    ///   the created session, allowing it to demux messages to the right
    ///   session when they are received in the future. The UDP protocol would
    ///   in turn want to `open_active` a session with the IP protocol, to which
    ///   it would pass itself as the invoker and {source_address,
    ///   destination_address} as the participant set.
    fn open_active(
        &mut self,
        invoker: ProtocolId,
        participants: ParticipantSet,
    ) -> Option<Box<dyn Session>>;

    /// Called by a lower-level protocol to open a session with a higher-level
    /// protocol when it does not recognize an incoming message as corresponding
    /// to an active session. This is useful for server programs listening for
    /// new connections.
    ///
    /// # Arguments
    ///
    /// * `invoker` is the protocol that requested to open a session. For
    ///   example, IP would be an invoker when it requests to open a new session
    ///   on UDP.
    /// * `participants` is an identifier for the session. For example, IP would
    ///   open a session with the participant set {source_address,
    ///   destination_address}.
    fn open_passive(
        &mut self,
        invoker: ProtocolId,
        participants: ParticipantSet,
    ) -> Option<Box<dyn Session>>;

    /// Allows a high-level protocol to request that messages for which there is
    /// no existing session be sent to it.
    ///
    /// # Arguments
    ///
    /// * `invoker` The protocol requesting to receive messages.
    /// * `participants` is the an identifier for the session. For example, both
    ///   TCP and UDP may want to have IP packets demuxed to them. TCP would ask
    ///   IP to add a demux binding to itself for {protocol_id: 6} while UDP
    ///   would ask to be bound to {protocol_id: 17}. Later, when IP receives a
    ///   packet with an unknown {source_address, destination_address} pair, it
    ///   can use the protocol field of the IP header to determine which
    ///   protocol should receive the message. It will then use `open_passive`
    ///   to create a new session with the corresponding protocol.
    fn add_demux_binding(&mut self, invoker: ProtocolId, participants: ParticipantSet);

    /// Identifies the session that a message belongs to.
    fn demux(&self, message: Message) -> Result<SessionId, Box<dyn Error>>;
}

/// A Session holds state for a particular connection. A Session "belongs"
/// to a Protocol.
pub trait Session {
    /// Returns the Protocol that this Session belongs to
    fn protocol(&self) -> ProtocolId;

    /// Invoked from a Protocol to send a Message.
    fn send(&self, message: Message, context: SessionContext) -> Result<(), Box<dyn Error>>;

    /// Invoked from a Protocol or Session object below for Message receipt.
    fn recv(&self, message: Message, context: SessionContext)
        -> Result<ProtocolId, Box<dyn Error>>;

    /// Invoked to allow the session to do some work it needs to do. For
    /// example, a TCP session may not be actively receiving or sending a
    /// message. However, it needs an opportunity to be woken up to advertise
    /// window sizes, retransmit data, poll a zero-sized window, or whatever
    /// else it may need to do.
    fn awake(&self, context: SessionContext);
}

pub struct SessionContext {}

impl SessionContext {
    pub fn send(&mut self, session_id: SessionId, message: Message) {
        todo!()
    }
}

pub type ParticipantSet = HashSet<(&'static str, Primitive)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
