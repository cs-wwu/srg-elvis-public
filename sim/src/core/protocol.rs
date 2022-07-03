use crate::core::Message;
use std::{
    error::Error,
    sync::{Arc, RwLock, Weak},
};
use thiserror::Error as ThisError;

use super::{Control, ProtocolId, ProtocolMap};

pub type ArcProtocol = Arc<RwLock<dyn Protocol>>;
pub type WeakProtocol = Weak<RwLock<dyn Protocol>>;
pub type ArcSession = Arc<RwLock<dyn Session>>;
pub type WeakSession = Weak<RwLock<dyn Session>>;

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
    // Todo: We need methods that allow other protocols to query info about a
    // protocol and its sessions. For example, a TCP or an IP protocol will want a
    // method to learn about a Nic's MTU.

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
        upstream: ProtocolId,
        participants: Control,
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
    fn listen(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    /// Identifies the session that a message belongs to.
    fn demux(
        &mut self,
        message: Message,
        // Todo: Can we remove this argument?
        downstream: ArcSession,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

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
    fn send(
        &mut self,
        self_handle: ArcSession,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    // Todo: We probably want demux to have already parsed the header and then pass
    // it on to the session. One of the things the x-kernel paper mentions is that
    // we want to touch the header as few times as possible for best performance. At
    // the moment, we require that the demux and recv methods each parse the header
    // separately, which is evidently inefficient. Without being able to make
    // Session generic, this does raise the question of what type would be
    // appropriate for passing structured header information from one method to
    // another. Do we possibly just attach information to the context? I'm not sure
    // just how efficient getting values from a HashMap is compared to just parsing
    // the header again. It's not entirely clear what to do here.

    /// Invoked from a Protocol or Session object below for Message receipt.
    fn recv(
        &mut self,
        self_handle: ArcSession,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    /// See [awake](elvis::core::Protocol::awake)
    fn awake(
        &mut self,
        self_handle: ArcSession,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;
}

#[derive(Clone, Default)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
    info: Control,
}

impl ProtocolContext {
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::default(),
        }
    }

    pub fn protocol(&self, id: ProtocolId) -> Result<ArcProtocol, ProtocolContextError> {
        self.protocols
            .get(&id)
            .cloned()
            .ok_or(ProtocolContextError::NoSuchProtocol)
    }

    pub fn info(&mut self) -> &mut Control {
        &mut self.info
    }
}

#[derive(Debug, ThisError)]
pub enum ProtocolContextError {
    #[error("Could not find the given protocol")]
    NoSuchProtocol,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}

pub enum ControlFlow {
    Continue,
    EndSimulation,
}
