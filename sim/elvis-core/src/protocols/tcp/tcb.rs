use super::{tcp_parsing::TcpHeader, ConnectionId};
use crate::Message;

#[derive(Debug, PartialEq, Eq, Hash, Default)]
pub struct Tcb {
    pub id: ConnectionId,
    pub state: State,
    pub snd: SendSequenceSpace,
    pub rcv: ReceiveSequenceSpace,
}

impl Tcb {
    pub fn closed() -> Self {
        Default::default()
    }

    pub fn listen() -> Self {
        Self {
            state: State::Listen,
            ..Default::default()
        }
    }

    pub fn open(&mut self, iss: u32, wnd: u16) {
        self.snd = SendSequenceSpace::new(iss, wnd);
    }

    pub fn send(&mut self, _message: Message) {
        todo!()
    }

    pub fn receive(&mut self, _header: TcpHeader, _message: Message) {
        todo!()
    }

    pub fn next_message(&mut self) -> Option<(TcpHeader, Message)> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum State {
    /// No connection.
    #[default]
    Closed,
    /// Waiting for a connection request from any remote TCP peer and port.
    Listen,
    /// Waiting for a matching connection request after having sent a connection
    /// request.
    SynSent,
    /// Waiting for a confirming connection request acknowledgment after having
    /// both received and sent a connection request.
    SynReceived,
    /// An open connection, data received can be delivered to the user. The
    /// normal state for the data transfer phase of the connection.
    Established,
    /// Waiting for a connection termination request from the remote TCP, or an
    /// acknowledgment of the connection termination request previously sent.
    FinWait1,
    /// Waiting for a connection termination request from the remote TCP.
    FinWait2,
    /// Waiting for a connection termination request from the local user.
    CloseWait,
    /// Waiting for a connection termination request acknowledgment from the
    /// remote TCP.
    Closing,
    /// Waiting for an acknowledgment of the connection termination request
    /// previously sent to the remote TCP (which includes an acknowledgment of
    /// its connection termination request).
    LastAck,
    /// Waiting for enough time to pass to be sure the remote TCP received the
    /// acknowledgment of its connection termination request.
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
#[derive(Debug, PartialEq, Eq, Hash, Default)]
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
    pub fn new(iss: u32, wnd: u16) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_3_5_fig_6() {
        // 1
        let mut peer_a = Tcb::closed();
        let mut peer_b = Tcb::listen();

        // 2
        peer_a.open(100, 4096);
        assert_eq!(peer_a.state, State::SynSent);
        let (header, message) = peer_a.next_message().unwrap();
        assert_eq!(header.seq, 100);
        assert!(header.ctl.syn());

        peer_b.receive(header, message);
        assert_eq!(peer_b.state, State::SynReceived);

        // 3
        let (header, message) = peer_b.next_message().unwrap();
        assert_eq!(header.seq, 300);
        assert_eq!(header.ack, 101);
        assert!(header.ctl.syn());
        assert!(header.ctl.ack());

        peer_a.receive(header, message);
        assert_eq!(peer_a.state, State::Established);

        // 4
        let (header, message) = peer_a.next_message().unwrap();
        assert_eq!(header.seq, 101);
        assert_eq!(header.ack, 301);
        assert!(header.ctl.ack());

        peer_b.receive(header, message);
        assert_eq!(peer_b.state, State::Established);

        // 5
        peer_a.send(Message::new("Hello!"));
        let (header, message) = peer_a.next_message().unwrap();
        assert_eq!(header.seq, 101);
        assert_eq!(header.ack, 301);
        assert!(header.ctl.ack());
        assert_eq!(message.len(), 6);

        peer_b.receive(header, message);
        assert_eq!(peer_b.state, State::Established);
    }
}
