/// The receive sequence space for TCP as described in [section
/// 3.3.1](https://www.rfc-editor.org/rfc/rfc9293.html#name-key-connection-state-variab)
/// of RFC 9293. It contains variables that describe a portion of the sequence
/// space related to incoming segments as described in the diagram below.
///
/// ```text
///     1          2          3
/// ----------|----------|----------
///        RCV.NXT    RCV.NXT
///                  +RCV.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers allowed for new reception
/// 3 - future sequence numbers which are not yet allowed
/// ```
/// Figure 4: Receive Sequence Space
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ReceiveSequenceSpace {
    /// Initial receive sequence number
    pub irs: u32,
    /// Next sequence number expected on an incoming segment, and is the
    /// left or lower edge of the receive window
    pub nxt: u32,
    /// The number of bytes we can buffer from the remote TCP
    pub wnd: u16,
}

impl Default for ReceiveSequenceSpace {
    fn default() -> Self {
        Self {
            irs: 0,
            nxt: 0,
            wnd: 4096,
        }
    }
}
