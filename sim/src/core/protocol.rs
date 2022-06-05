use crate::core::Message;

/// Protocols are stackable objects that function as network processing elements.
/// Protocols have Protocols stacked above them and Protocols stacked below them.
/// `set_up` and `set_down` are used to create the stacking order.
///
/// Invoke `open` on a Protocol to create a Session object.
/// A Protocol maintains a list of Session objects that encapsulate connection state.
///
/// Protcols expose methods to send and receive Messages.
///
/// # Receive Path
///
/// A Protocol receives Messages via a `recv` method from below.
/// The Message header is examined to determine the appropriate Session object.
/// The session's `recv` method is called to route the message appropriately.
/// The Session object may strip headers, and then call `recv` on a higher level Protocol.
///
/// # Send Path
///
/// A Session is invoked with a `send` method from above.
/// The Session may add headers, and send the message onward to the Protocol object below.
/// The Protocol object is expected to demux the message to the right Session,
/// and invoke the Sessions's `send` method.
pub trait Protocol {
    /// Return an identifier for the protocol. Identifiers are 32 bit constants
    /// statically assigned throughout the simulation. This simplifies
    /// Protocols/Sessions demultiplexing to the right protocol on message receipt
    fn id(&self) -> i32;

    /// Stack the given protocol above this one
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to stack above this one
    fn set_up(&self, protocol: &dyn Protocol);

    /// Stack this protocol above the given one
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to stacked below this one
    fn set_down(&self, protocol: &dyn Protocol);

    /// Invoked from above to create a Session
    ///
    /// # Arguments
    ///
    /// * `args` - A byte array as arguments to the open call
    ///
    /// # Returns
    ///
    /// A Session object
    fn open(&self, args: &[u8]) -> dyn Session;

    /// Invoked from a Protocol to send a Message.
    ///
    /// # Arguments
    ///
    /// * `message` - The Message to send. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn send(&self, message: Message) -> bool;

    /// Invoked from a Protocol or Session object below for Message receipt.
    /// The Protocol is expected to demultiplex this message to the correct
    /// Session object at this layer. The message is passed to the Session
    /// by calling `Session::recv()`
    ///
    /// # Arguments
    ///
    /// * `sender` - The Session that is sending this message
    /// * `message` - The Message to receive. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn recv(&self, sender: &dyn Session, message: Message) -> i32;

    /// Return the list of stacked protocols above
    fn above(&self) -> &Vec<&dyn Protocol>;

    /// Return the list of stacked protocols below
    fn below(&self) -> &Vec<&dyn Protocol>;
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
    /// * `message` - The Message to send. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn send(&self, message: Message) -> bool;

    /// Invoked from a Protocol or Session object below for Message receipt.
    ///
    /// # Arguments
    ///
    /// * `sender` - The lower level session that sent this message
    /// * `message` - The Message to receive. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn recv(&self, sender: &dyn Session, message: Message) -> bool;
}
