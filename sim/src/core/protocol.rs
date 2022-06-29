use crate::core::Message;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock, Weak},
};
use thiserror::Error as ThisError;

use super::ProtocolMap;

pub type ArcProtocol = Arc<RwLock<dyn Protocol>>;
pub type WeakProtocol = Weak<RwLock<dyn Protocol>>;
pub type ArcSession = Arc<RwLock<dyn Session>>;
pub type WeakSession = Weak<RwLock<dyn Session>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolId {
    layer: NetworkLayer,
    // Todo: If there are many user programs, this should probably be a larger primitive
    identifier: u8,
}

impl ProtocolId {
    pub const fn new(layer: NetworkLayer, identifier: u8) -> Self {
        Self { layer, identifier }
    }
}

impl From<ProtocolId> for [u8; 2] {
    fn from(id: ProtocolId) -> Self {
        [id.layer as u8, id.identifier]
    }
}

impl From<ProtocolId> for u16 {
    fn from(id: ProtocolId) -> Self {
        let bytes: [u8; 2] = id.into();
        Self::from_be_bytes(bytes)
    }
}

impl TryFrom<[u8; 2]> for ProtocolId {
    type Error = NetworkLayerError;

    fn try_from(value: [u8; 2]) -> Result<Self, Self::Error> {
        Ok(Self {
            layer: value[0].try_into()?,
            identifier: value[1],
        })
    }
}

impl TryFrom<u16> for ProtocolId {
    type Error = NetworkLayerError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        value.to_be_bytes().try_into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NetworkLayer {
    Link,
    Network,
    Transport,
    Application,
    User,
}

impl TryFrom<u8> for NetworkLayer {
    type Error = NetworkLayerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NetworkLayer::Link),
            1 => Ok(NetworkLayer::Network),
            2 => Ok(NetworkLayer::Transport),
            3 => Ok(NetworkLayer::Application),
            _ => Err(NetworkLayerError::FromByte(value)),
        }
    }
}

#[derive(Debug, ThisError)]
pub enum NetworkLayerError {
    #[error("Unable to create a network layer from the byte {0}")]
    FromByte(u8),
}

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
    /// * `requester` is the higher-level protocol that requested to open the
    ///   session.
    /// * `identifier` is an identifier for the session. For example, one might
    ///   open a session on a UDP protocol with the participant set
    ///   {source_address, source_port, destination_address, destination_port}.
    ///   The UDP protocol would then save a mapping of this participant set to
    ///   the created session, allowing it to demux messages to the right
    ///   session when they are received in the future. The UDP protocol would
    ///   in turn want to `open_active` a session with the IP protocol, to which
    ///   it would pass itself as the requester and {source_address,
    ///   destination_address} as the participant set.
    fn open_active(
        &mut self,
        requester: ArcProtocol,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>>;

    /// Called by a lower-level protocol to open a session with a higher-level
    /// protocol when it does not recognize an incoming message as corresponding
    /// to an active session. This is useful for server programs listening for
    /// new connections.
    ///
    /// # Arguments
    ///
    /// * `requester` is the protocol that requested to open a session. For
    ///   example, IP would be an requester when it requests to open a new
    ///   session on UDP.
    /// * `identifier` is an identifier for the session. For example, IP would
    ///   open a session with the participant set {source_address,
    ///   destination_address}.
    fn open_passive(
        &mut self,
        requester: ArcSession,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>>;

    /// Allows a high-level protocol to request that messages for which there is
    /// no existing session be sent to it.
    ///
    /// # Arguments
    ///
    /// * `requester` The protocol requesting to receive messages.
    /// * `identifier` is the an identifier for the session. For example, both
    ///   TCP and UDP may want to have IP packets demuxed to them. TCP would ask
    ///   IP to add a demux binding to itself for {protocol_id: 6} while UDP
    ///   would ask to be bound to {protocol_id: 17}. Later, when IP receives a
    ///   packet with an unknown {source_address, destination_address} pair, it
    ///   can use the protocol field of the IP header to determine which
    ///   protocol should receive the message. It will then use `open_passive`
    ///   to create a new session with the corresponding protocol. As another
    ///   example, suppose that a user program wants to listen for unknown TCP
    ///   connections. It can request that the TCP protocol add a demux binding
    ///   for {local_port}. When TCP receives a message on that port, it will
    ///   passively open a session with the user program and the user program
    ///   will see that as a new connection.
    fn add_demux_binding(
        &mut self,
        requester: ArcProtocol,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    /// Identifies the session that a message belongs to.
    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>>;

    /// Invoked to allow the protocol to do some work it needs to do. For
    /// example, a TCP session may not be actively receiving or sending a
    /// message. However, it needs an opportunity to be woken up to advertise
    /// window sizes, retransmit data, poll a zero-sized window, or whatever
    /// else it may need to do.
    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>>;
}

/// A Session holds state for a particular connection. A Session "belongs"
/// to a Protocol.
pub trait Session {
    // Returns the ID of the protocol used to demux messages upward
    fn protocol(&self) -> ProtocolId;

    /// Invoked from a Protocol to send a Message.
    fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>>;

    /// Invoked from a Protocol or Session object below for Message receipt.
    fn recv(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>>;

    /// See [awake](elvis::core::Protocol::awake)
    fn awake(&mut self, context: ProtocolContext) -> Result<(), Box<dyn Error>>;
}

#[derive(Clone)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
}

impl ProtocolContext {
    pub fn new(protocols: ProtocolMap) -> Self {
        Self { protocols }
    }

    pub fn protocol(&self, id: ProtocolId) -> Option<ArcProtocol> {
        self.protocols.get(&id).cloned()
    }
}

pub enum ControlFlow {
    Continue,
    EndSimulation,
}

pub type Control = HashMap<ControlKey, Primitive>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKey {
    SourceAddress,
    DestinationAddress,
    SourcePort,
    DestinationPort,
    Protocol(ProtocolId),
    NetworkIndex,
    Other(&'static str),
}

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

impl From<u8> for Primitive {
    fn from(value: u8) -> Self {
        Self::U8(value)
    }
}

impl From<u16> for Primitive {
    fn from(value: u16) -> Self {
        Self::U16(value)
    }
}

impl From<u32> for Primitive {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

impl From<u64> for Primitive {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl From<u128> for Primitive {
    fn from(value: u128) -> Self {
        Self::U128(value)
    }
}

impl From<i8> for Primitive {
    fn from(value: i8) -> Self {
        Self::I8(value)
    }
}

impl From<i16> for Primitive {
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}

impl From<i32> for Primitive {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<i64> for Primitive {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<i128> for Primitive {
    fn from(value: i128) -> Self {
        Self::I128(value)
    }
}
