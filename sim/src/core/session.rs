use super::{Message, ProtocolContext, ProtocolId};
use std::{cell::RefCell, error::Error, rc::Rc};

pub type RcSession = Rc<RefCell<dyn Session>>;

/// A Session holds state for a particular connection. A Session "belongs"
/// to a Protocol.
pub trait Session {
    // Returns the ID of the protocol used to demux messages upward
    fn protocol(&self) -> ProtocolId;

    /// Invoked from a Protocol to send a Message.
    fn send(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
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
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;

    fn awake(
        &mut self,
        self_handle: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>>;
}

pub enum ControlFlow {
    Continue,
    EndSimulation,
}
