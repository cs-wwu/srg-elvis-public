use super::{tcp_parsing::TcpHeader, ConnectionId, Iss};
use crate::Message;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Tcb {
    pub state: State,
    pub id: ConnectionId,
    pub send: SendSequenceSpace,
    pub recv: ReceiveSequenceSpace,
}

impl Tcb {
    pub fn receive(&mut self, _message: Message, _header: TcpHeader) {
        todo!()
    }

    pub fn poll_send(&mut self) -> Option<Message> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// Represents waiting for a matching connection request after having sent a
    /// connection request.
    SynSent,
    /// Represents waiting for a confirming connection request acknowledgment
    /// after having both received and sent a connection request.
    SynReceived,
    /// Represents an open connection, data received can be delivered to the
    /// user. The normal state for the data transfer phase of the connection.
    Established,
    /// Represents waiting for a connection termination request from the remote
    /// TCP, or an acknowledgment of the connection termination request
    /// previously sent.
    FinWait1,
    /// Represents waiting for a connection termination request from the remote
    /// TCP.
    FinWait2,
    /// Represents waiting for a connection termination request from the local
    /// user.
    CloseWait,
    /// Represents waiting for a connection termination request acknowledgment
    /// from the remote TCP.
    Closing,
    /// Represents waiting for an acknowledgment of the connection termination
    /// request previously sent to the remote TCP (which includes an
    /// acknowledgment of its connection termination request).
    LastAck,
    /// Represents waiting for enough time to pass to be sure the remote TCP
    /// received the acknowledgment of its connection termination request.
    TimeWait,
}

//      1         2          3          4
// ----------|----------|----------|----------
//        SND.UNA    SND.NXT    SND.UNA
//                             +SND.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers of unacknowledged data
// 3 - sequence numbers allowed for new data transmission (send window)
// 4 - future sequence numbers which are not yet allowed
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SendSequenceSpace {
    /// Unacknowledged
    pub una: u32,
    /// Next
    pub nxt: u32,
    /// Window
    pub wnd: u16,
    /// Segment sequence number used for last window update
    pub wl1: u32,
    /// Segment acknowledgment number used for last window update
    pub wl2: u32,
    /// Initial sequence number
    pub iss: u32,
}

impl SendSequenceSpace {
    pub fn new(iss: Iss, wnd: u16) -> Self {
        let iss = iss.into();
        Self {
            iss,
            una: iss,
            nxt: iss + 1,
            wnd,
            wl1: 0,
            wl2: 0,
        }
    }
}

//     1          2          3
// ----------|----------|----------
//        RCV.NXT    RCV.NXT
//                  +RCV.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers allowed for new reception
// 3 - future sequence numbers which are not yet allowed
#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct ReceiveSequenceSpace {
    /// Next
    pub nxt: u32,
    /// Window
    pub wnd: u16,
    /// Initial receive sequence
    pub irs: u32,
}

impl ReceiveSequenceSpace {
    pub fn new() -> Self {
        Default::default()
    }
}

/// Is `b` between `a` and `c` when accounting for modular arithmetic?
fn is_between_wrapped(a: u32, b: u32, c: u32) -> bool {
    (a < b && b < c) || (c < a && a < b) || (b < c && c < a)
}
