use crate::core::Message;

/// Protocols are stackable objects that function as network processing elements.
/// Protocols have Protocols stacked above them and Protocols stacked below them.
/// Protcols expose methods to send and receive Messages.
/// A Protocol maintains a list of Session objects that encapsulate connection state.
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
pub trait Protocol {
    /// Return an identifier for the protocol. Identifiers are 32 bit constants
    /// statically assigned throughout the simulation. This simplifies
    /// Protocols/Sessions demultiplexing to the right protocol on message receipt
    fn id(&self) -> i32;

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

    /// Invoked from a Protocol or Session object below to for Message receipt.
    ///
    /// # Arguments
    ///
    /// * `message` - The Message to receive. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn recv(&self, message: Message) -> i32;

    /// Invoked from above to create a Session
    ///
    /// # Arguments
    ///
    /// * `token` - A token of type T to parameterize the open
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn open(&self) -> dyn Session;

    /// Return the list of stacked protocols above
    fn above(&self) -> &Vec<&dyn Protocol>;

    /// Return the list of stacked protocols below
    fn below(&self) -> &Vec<&dyn Protocol>;

    /// Set the given protocol to be above this one
    fn set_up(&self, protocol: &dyn Protocol);

    /// Set the given protocol to be below this one
    fn set_down(&self, protocol: &dyn Protocol);
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

    /// Invoked from a Protocol or Session object below to for Message receipt.
    ///
    /// # Arguments
    ///
    /// * `message` - The Message to receive. Ownership passes to the protocol
    ///
    /// # Returns
    ///
    /// 0 on success, or a non-zero error code on failure
    fn recv(&self, message: Message) -> bool;
}
