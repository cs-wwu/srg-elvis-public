use super::{
    tcp_parsing::{BuildHeaderError, TcpHeader, TcpHeaderBuilder},
    ConnectionId,
};
use crate::{
    protocols::{ipv4::Ipv4Address, utility::Socket},
    Message,
};
use std::{
    collections::{BinaryHeap, VecDeque},
    time::Duration,
};

// NOTE(hardint): Section numbers are base on RFC 9293, the updated TCP protocol
// specification

// NOTE(hardint): In the current implementation, we respond to with ACKs
// immediately instead of trying to piggyback them for efficiency. This can be revised once the implementation is working.

// TODO(hardint): Do actual window management
const RCV_WND: u16 = 4096;

// TODO(hardint): Choose a more realistic value
/// The maximum segment lifetime on the Internet
const MSL: Duration = Duration::new(1, 0);

// TODO(hardint): Choose a better value
/// The time that may pass before packets are retransmitted
const RETRANSMISSION_TIMEOUT: Duration = Duration::new(1, 0);

#[derive(Debug, Clone)]
pub struct Tcb {
    id: ConnectionId,
    initiation: Initiation,
    state: State,
    snd: SendSequenceSpace,
    rcv: ReceiveSequenceSpace,
    outgoing: VecDeque<(TcpHeader, Message)>,
    incoming: BinaryHeap<Incoming>,
    retransmission_timeout: Option<Duration>,
    time_wait_timeout: Option<Duration>,
}

impl Tcb {
    fn new(
        id: ConnectionId,
        initiation: Initiation,
        state: State,
        snd: SendSequenceSpace,
        rcv: ReceiveSequenceSpace,
    ) -> Self {
        Self {
            id,
            initiation,
            state,
            snd,
            rcv,
            outgoing: Default::default(),
            incoming: Default::default(),
            retransmission_timeout: None,
            time_wait_timeout: None,
        }
    }

    pub fn open(id: ConnectionId, iss: u32) -> Self {
        // 3.10.1 Specifically for the case of an active open. Handling for
        // packets in a passive open LISTEN state is provided in a freestanding
        // function.
        let mut tcb = Self::new(
            id,
            Initiation::Open,
            State::SynSent,
            SendSequenceSpace {
                iss,
                una: iss,
                nxt: iss + 1,
                ..Default::default()
            },
            ReceiveSequenceSpace::default(),
        );
        tcb.enqueue_outgoing(tcb.header_builder(iss).syn(), [].into())
            .unwrap();
        tcb
    }

    pub fn advance_time(&mut self, delta_time: Duration) -> AdvanceTimeResult {
        if let Some(retransmission) = self.retransmission_timeout {
            if delta_time > retransmission {
                self.retransmission_timeout = Some(RETRANSMISSION_TIMEOUT);
                // TODO(hardint): Retransmit data
                todo!()
            } else {
                self.retransmission_timeout = Some(retransmission - delta_time);
            }
        }

        if let Some(time_wait) = self.time_wait_timeout {
            if delta_time > time_wait {
                return AdvanceTimeResult::CloseConnection;
            }
            self.time_wait_timeout = Some(time_wait - delta_time);
        }

        AdvanceTimeResult::Ignore
    }

    pub fn send(&mut self, _message: Message) -> Result<(), BuildHeaderError> {
        // 3.10.2
        todo!()
    }

    pub fn receive(&mut self) {
        // 3.10.3
        // if !self.is_seq_ok(message.len() as u32, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
        //     self.enqueue_outgoing(
        //         self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
        //         [].into(),
        //     )?;
        //     return Ok(ReceiveResult::DiscardSegment);
        // }
        todo!()
    }

    pub fn close(&mut self) {
        // 3.10.4
        todo!()
    }

    pub fn abort(&mut self) {
        // 3.10.5
        todo!()
    }

    pub fn status(&mut self) {
        // 3.10.6
        todo!()
    }

    pub fn segment_arrives(
        &mut self,
        seg: TcpHeader,
        message: Message,
    ) -> Result<ReceiveResult, ReceiveError> {
        if seg.ctl.ack() {
            match self.state {
                State::SynSent => {
                    if seg.ctl.rst() {
                        // Discard the segment
                        return Ok(ReceiveResult::DiscardSegment);
                    }

                    if mod_bounded(self.snd.nxt, Le, seg.ack, Leq, self.snd.iss)
                        || !mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt)
                    {
                        // Send a reset and discard the segment
                        self.enqueue_outgoing(self.header_builder(seg.ack).rst(), [].into())?;
                        return Ok(ReceiveResult::InvalidAck);
                    }
                }

                State::SynReceived => {
                    if mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                        self.state = State::Established;
                        self.snd.wnd = seg.wnd;
                        self.snd.wl1 = seg.seq;
                        self.snd.wl2 = seg.ack;
                    } else {
                        self.enqueue_outgoing(self.header_builder(seg.ack).rst(), [].into())?;
                    }
                }

                // ESTABLISHED processing:
                State::Established
                | State::FinWait1
                | State::FinWait2
                | State::CloseWait
                | State::Closing => {
                    if mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                        self.snd.una = seg.ack;
                        // TODO(hardint): Remove acknowledged segments from retransmission queue
                    }

                    if mod_bounded(self.snd.una, Leq, seg.ack, Leq, self.snd.nxt) {
                        // Update the send window
                        if mod_le(self.snd.wl1, seg.seq)
                            || (self.snd.wl1 == seg.seq && mod_leq(self.snd.wl2, seg.ack))
                        {
                            self.snd.wnd = seg.wnd;
                            self.snd.wl1 = seg.seq;
                            self.snd.wl2 = seg.ack;
                        }
                    }

                    // In addition to ESTABLISHED processing:
                    match self.state {
                        State::FinWait1 => {
                            // TODO(hardint): If the FIN segment is now acknowledged,
                            // enter FIN-WAIT-2 and continue processing
                        }

                        State::FinWait2 => {
                            // TODO(hardint): If the retransmission
                            // queue is empty, acknowledge the user's
                            // CLOSE
                        }

                        State::Closing => {
                            // TODO(hardint): If the ACK acknowledges
                            // our FIN, enter TIME-WAIT.
                        }

                        _ => {}
                    }
                }

                State::LastAck => {
                    // TODO(hardint): If our FIN is now acknowledged,
                    // close the TCB
                }

                State::TimeWait => {
                    // TODO(hardint): The only thing that can arrive is
                    // a retransmission of the remote FIN. Acknowledge
                    // it and restart the MSL 2 timeout.
                }
            }
        }

        if seg.ctl.rst() {
            match self.state {
                State::SynSent => {
                    if seg.seq == self.rcv.nxt {
                        return Ok(ReceiveResult::ConnectionReset);
                    } else {
                        return Err(ReceiveError::BlindReset);
                    };
                }

                State::SynReceived => match self.initiation {
                    Initiation::Listen => {
                        return Ok(ReceiveResult::ReturnToListen);
                    }
                    Initiation::Open => {
                        return Ok(ReceiveResult::ConnectionRefused);
                    }
                },

                State::Established | State::FinWait1 | State::FinWait2 | State::CloseWait => {
                    // TODO(hardint): Outstanding RECEIVEs and SENDs
                    // should receive reset responses.
                    return Ok(ReceiveResult::ConnectionReset);
                }

                State::Closing | State::LastAck | State::TimeWait => {
                    return Ok(ReceiveResult::FinalizeClose);
                }
            }
        }

        if seg.ctl.syn() {
            match self.state {
                State::SynSent => {
                    self.rcv.irs = seg.seq;
                    self.rcv.nxt = seg.seq + 1;

                    // TODO(hardint): Remove acknowledged segments from the
                    // retransmission queue

                    // FIX(hardint): Only advance, and only if this segment is an ACK
                    self.snd.una = seg.ack;

                    if mod_ge(self.snd.una, self.snd.iss) {
                        self.state = State::Established;
                        self.enqueue_outgoing(
                            self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                            [].into(),
                        )?;
                    } else {
                        self.state = State::SynReceived;
                        self.enqueue_outgoing(
                            self.header_builder(self.snd.iss).syn().ack(self.rcv.nxt),
                            [].into(),
                        )?;
                        self.snd.wnd = seg.wnd;
                        self.snd.wl1 = seg.seq;
                        self.snd.wl2 = seg.ack;
                        // TODO(hardint): Queue other controls or text for
                        // processing in Established state

                        return Ok(ReceiveResult::Success);
                    }
                }

                State::Established
                | State::FinWait1
                | State::FinWait2
                | State::CloseWait
                | State::Closing
                | State::LastAck
                | State::TimeWait
                | State::SynReceived => {
                    if self.state == State::SynReceived && self.initiation == Initiation::Listen {
                        return Ok(ReceiveResult::ReturnToListen);
                    }

                    // Getting a SYN in a synchronized state is weird. In
                    // following with RFC 5961, send a challenge ACK and stop
                    // further processing:
                    self.enqueue_outgoing(
                        self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                        [].into(),
                    )?;
                    return Ok(ReceiveResult::DiscardSegment);
                }
            }
        }

        // Queue the segment text for processing
        match self.state {
            State::Established | State::FinWait1 | State::FinWait2 => {
                self.incoming.push(Incoming::new(seg, message));
            }

            State::SynSent
            | State::SynReceived
            | State::CloseWait
            | State::Closing
            | State::LastAck
            | State::TimeWait => {
                // TODO(hardint)
            }
        }

        if seg.ctl.fin() {
            match self.state {
                State::SynSent | State::CloseWait | State::Closing | State::LastAck => {}

                State::SynReceived | State::Established => {
                    self.state = State::CloseWait;
                }

                State::FinWait1 => {
                    // TODO(hardint): If our FIN has been ACKed (perhaps in this
                    // segment), then enter TIME-WAIT, start the time-wait
                    // timer, turn off the other timers; otherwise, enter the
                    // CLOSING state.
                }

                State::FinWait2 => {
                    self.state = State::TimeWait;
                    // TODO(hardint): Start the time-wait timer, turn off the
                    // other timers.
                }

                State::TimeWait => {
                    // TODO(hardint): Restart the 2 MSL time-wait timeout.
                }
            }
        }

        Ok(ReceiveResult::Success)
    }

    fn header_builder(&self, seq: u32) -> TcpHeaderBuilder {
        TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seq)
    }

    fn enqueue_outgoing(
        &mut self,
        header_builder: TcpHeaderBuilder,
        message: Message,
    ) -> Result<(), BuildHeaderError> {
        let header = header_builder.build(
            self.id.local.address,
            self.id.remote.address,
            message.iter(),
        )?;
        self.outgoing.push_back((header, message));
        Ok(())
    }

    fn is_ack_ok(&self, ack: u32) -> bool {
        mod_bounded(self.snd.una, Le, ack, Leq, self.snd.nxt)
    }

    fn is_seq_ok(&self, data_len: u32, seq: u32, syn: bool, fin: bool) -> bool {
        let seg_len = data_len + fin as u32 + syn as u32;
        // Test segment acceptability. See Table 6.
        if seg_len == 0 {
            if RCV_WND == 0 {
                seq == self.rcv.nxt
            } else {
                self.is_in_window(seq)
            }
        } else if RCV_WND == 0 {
            // When the receive window is zero, only ACKs are acceptible.
            false
        } else {
            self.is_in_window(seq) || self.is_in_window(seq + seg_len - 1)
        }
    }

    fn is_in_window(&self, n: u32) -> bool {
        mod_bounded(self.rcv.nxt, Leq, n, Le, self.rcv.nxt + RCV_WND as u32)
    }
}

pub fn handle_closed(
    seg: TcpHeader,
    // Specifically the length of the payload. Does not count the header.
    seg_len: u32,
    local: Ipv4Address,
    remote: Ipv4Address,
) -> Option<TcpHeader> {
    // 3.10.7.1
    if seg.ctl.rst() {
        // Discard RST segments
        return None;
    }

    if seg.ctl.ack() {
        TcpHeaderBuilder::new(seg.dst_port, seg.src_port, seg.ack).rst()
    } else {
        TcpHeaderBuilder::new(seg.dst_port, seg.src_port, 0)
            .rst()
            .ack(seg.seq + seg_len)
    }
    .build(local, remote, [].into_iter())
    .ok()
}

pub fn handle_listen(
    mut seg: TcpHeader,
    message: Message,
    local: Ipv4Address,
    remote: Ipv4Address,
    iss: u32,
) -> Option<ListenResult> {
    // 3.10.7.2
    if seg.ctl.rst() {
        // First:
        // Could not be valid, ignore
        return None;
    }

    if seg.ctl.ack() {
        // Second:
        // Bad acknowledgement, reset
        TcpHeaderBuilder::new(seg.dst_port, seg.src_port, seg.ack)
            .rst()
            .build(local, remote, [].into_iter())
            .ok()
            .map(ListenResult::Response)
    } else if seg.ctl.syn() {
        // Third:
        // NOTE: Ignore security check for simplicity
        let mut tcb = Tcb::new(
            ConnectionId {
                local: Socket {
                    address: local,
                    port: seg.dst_port,
                },
                remote: Socket {
                    address: remote,
                    port: seg.src_port,
                },
            },
            Initiation::Listen,
            State::SynReceived,
            SendSequenceSpace {
                iss,
                una: iss,
                nxt: iss + 1,
                wnd: seg.wnd,
                wl1: seg.seq,
                wl2: seg.ack,
            },
            ReceiveSequenceSpace {
                irs: seg.seq,
                nxt: seg.seq + 1,
            },
        );
        tcb.enqueue_outgoing(tcb.header_builder(iss).syn().ack(tcb.rcv.nxt), [].into())
            .ok()?;

        // Processing of SYN and ACK should not be repeated.

        // NOTE(hardint): At the moment, ACKs are sent immediately when a
        // segment arrives rather than trying to defer and piggyback, so this
        // doesn't really matter as we don't reprocess ACKing segments after
        // they arrive. Still, setting these fields is here for completeness.
        seg.ctl.set_syn(false);
        seg.ctl.set_ack(false);
        tcb.incoming.push(Incoming::new(seg, message));

        Some(ListenResult::Tcb(tcb))
    } else {
        // Fourth:
        // Any other control or data-bearing segment should be discarded
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
struct SendSequenceSpace {
    /// Unacknowledged
    una: u32,
    /// Next
    nxt: u32,
    /// Window
    wnd: u16,
    /// Segment sequence number used for last window update
    wl1: u32,
    /// Segment acknowledgment number used for last window update
    wl2: u32,
    /// Initial sequence number
    iss: u32,
}

//     1          2          3
// ----------|----------|----------
//        RCV.NXT    RCV.NXT
//                  +RCV.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers allowed for new reception
// 3 - future sequence numbers which are not yet allowed
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
struct ReceiveSequenceSpace {
    /// Initial receive sequence
    irs: u32,
    /// Next
    nxt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveResult {
    Success,
    DiscardSegment,
    InvalidAck,
    UnacceptableSegment,
    ReturnToListen,
    ConnectionReset,
    ConnectionRefused,
    FinalizeClose,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("{0}")]
    Header(#[from] BuildHeaderError),
    #[error("SEG.RST and RCV.NXT != SEG.SEQ")]
    BlindReset,
}

#[derive(Debug, Clone)]
struct Incoming {
    seg: TcpHeader,
    message: Message,
}

impl Incoming {
    pub fn new(seg: TcpHeader, message: Message) -> Self {
        Self { seg, message }
    }
}

impl PartialEq for Incoming {
    fn eq(&self, other: &Self) -> bool {
        self.seg.seq == other.seg.seq
    }
}

impl Eq for Incoming {}

impl PartialOrd for Incoming {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Incoming {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seg.seq.cmp(&other.seg.seq)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Initiation {
    Listen,
    Open,
}

pub enum ListenResult {
    Response(TcpHeader),
    Tcb(Tcb),
}

impl ListenResult {
    fn response(self) -> Option<TcpHeader> {
        match self {
            ListenResult::Response(response) => Some(response),
            ListenResult::Tcb(_) => None,
        }
    }

    fn tcb(self) -> Option<Tcb> {
        match self {
            ListenResult::Response(_) => None,
            ListenResult::Tcb(tcb) => Some(tcb),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceTimeResult {
    Ignore,
    CloseConnection,
}

/// a < b under modular arithmetic
fn mod_le(a: u32, b: u32) -> bool {
    // k is on the opposite side of the ring of integers mod 32 from b
    let k = b.wrapping_add(u32::MAX / 2);

    // There are six cases:
    //  0123456789
    // |a b    k  | a<b, a<k, b<k -> a<b
    // |a k    b  | a<b, a<k, b>k -> a>b
    // |  b a  k  | a>b, a<k, b<k -> a>b
    // |  k a  b  | a<b, a>k, b>k -> a<b
    // |  b    k a| a>b, a>k, b<k -> a<b
    // |  k    b a| a>b, a>k, b>k -> a>b

    (a < b) ^ (a < k) ^ (b < k)
}

/// a <= b under modular arithmetic
fn mod_leq(a: u32, b: u32) -> bool {
    mod_le(a, b.wrapping_add(1))
}

/// a > b under modular arithmetic
fn mod_ge(a: u32, b: u32) -> bool {
    mod_le(b, a)
}

/// a > b under modular arithmetic
fn mod_geq(a: u32, b: u32) -> bool {
    mod_le(b.wrapping_sub(1), a)
}

use ModCmp::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModCmp {
    Le,
    Leq,
}

impl ModCmp {
    pub fn offset(self) -> u32 {
        match self {
            Le => 0,
            Leq => 1,
        }
    }
}

/// Is `b` between `a` and `c` when accounting for modular arithmetic?
fn mod_bounded(a: u32, ab_cmp: ModCmp, b: u32, bc_cmp: ModCmp, c: u32) -> bool {
    let a = a.wrapping_sub(ab_cmp.offset());
    let c = c.wrapping_add(bc_cmp.offset());

    // a < b < c holds under the following conditions:
    // j: | a b c |
    // k: | c a b |
    // l: | b c a |

    let j = a < b && b < c && a < c;
    let k = a < b && b > c && a > c;
    let l = a > b && b < c && a > c;
    j || k || l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modular_comparison() {
        // 2**31 = 2_147_483_648
        assert!(mod_le(10, 20));
        assert!(!mod_le(20, 10));
        assert!(mod_le(2_000_000_000, 3_000_000_000));
        assert!(!mod_le(3_000_000_000, 2_000_000_000));
        assert!(mod_le(3_000_000_000, 4_000_000_000));
        assert!(!mod_le(4_000_000_000, 3_000_000_000));

        assert!(!mod_le(5, 5));
        assert!(mod_leq(5, 5));

        assert!(mod_ge(20, 10));
        assert!(!mod_ge(5, 5));
        assert!(mod_geq(5, 5));

        assert!(mod_bounded(5, Le, 10, Le, 15));
        assert!(!mod_bounded(15, Le, 10, Le, 5));

        assert!(mod_bounded(u32::MAX - 5, Le, 5, Le, 10));
        assert!(!mod_bounded(10, Le, 5, Le, u32::MAX - 5));

        assert!(mod_bounded(u32::MAX - 10, Le, u32::MAX - 5, Le, 5));
        assert!(!mod_bounded(5, Le, u32::MAX - 5, Le, u32::MAX - 10));

        assert!(!mod_bounded(5, Le, 5, Le, 15));
        assert!(mod_bounded(5, Leq, 5, Le, 15));
        assert!(!mod_bounded(5, Le, 15, Le, 15));
        assert!(mod_bounded(5, Le, 15, Leq, 15));
        assert!(mod_bounded(10, Leq, 10, Leq, 10));
    }

    const PEER_A_ID: ConnectionId = ConnectionId {
        local: Socket {
            address: Ipv4Address::new([0, 0, 0, 0]),
            port: 0xcafe,
        },
        remote: Socket {
            address: Ipv4Address::new([0, 0, 0, 1]),
            port: 0xdead,
        },
    };

    const PEER_B_ID: ConnectionId = PEER_A_ID.reverse();

    #[test]
    fn basic_synchronization() {
        // Based on 3.5 Figure 6:
        //
        //     TCP Peer A                                            TCP Peer B
        // 1.  CLOSED                                                LISTEN
        // 2.  SYN-SENT    --> <SEQ=100><CTL=SYN>                --> SYN-RECEIVED
        // 3.  ESTABLISHED <-- <SEQ=300><ACK=101><CTL=SYN,ACK>   <-- SYN-RECEIVED
        // 4.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK>       --> ESTABLISHED
        // 5.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK><DATA> --> ESTABLISHED

        // 2
        let mut peer_a = Tcb::open(PEER_A_ID, 100);
        assert_eq!(peer_a.state, State::SynSent);
        let (header, message) = peer_a.outgoing.pop_back().unwrap();
        assert_eq!(header.seq, 100);
        assert!(header.ctl.syn());

        let mut peer_b = handle_listen(
            header,
            message,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
        )
        .unwrap()
        .tcb()
        .unwrap();
        assert_eq!(peer_b.state, State::SynReceived);

        // 3
        let (header, message) = peer_b.outgoing.pop_back().unwrap();
        assert_eq!(header.seq, 300);
        assert_eq!(header.ack, 101);
        assert!(header.ctl.syn());
        assert!(header.ctl.ack());

        peer_a.segment_arrives(header, message).unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 4
        let (header, message) = peer_a.outgoing.pop_back().unwrap();
        assert_eq!(header.seq, 101);
        assert_eq!(header.ack, 301);
        assert!(header.ctl.ack());

        peer_b.segment_arrives(header, message).unwrap();
        assert_eq!(peer_b.state, State::Established);

        // 5 TODO(hardint): Needs data segment transmission to wore
    }

    #[test]
    fn simultaneous_initiation() {
        // Based on 3.5 Figure 7:
        //
        //     TCP Peer A                                       TCP Peer B
        // 1.  CLOSED                                           CLOSED
        // 2.  SYN-SENT     --> <SEQ=100><CTL=SYN>              ...
        // 3.  SYN-RECEIVED <-- <SEQ=300><CTL=SYN>              <-- SYN-SENT
        // 4.               ... <SEQ=100><CTL=SYN>              --> SYN-RECEIVED
        // 5.  SYN-RECEIVED --> <SEQ=100><ACK=301><CTL=SYN,ACK> ...
        // 6.  ESTABLISHED  <-- <SEQ=300><ACK=101><CTL=SYN,ACK> <-- SYN-RECEIVED
        // 7.               ... <SEQ=100><ACK=301><CTL=SYN,ACK> --> ESTABLISHED

        // 2
        let mut peer_a = Tcb::open(PEER_A_ID, 100);
        assert_eq!(peer_a.state, State::SynSent);
        let a_syn = peer_a.outgoing.pop_back().unwrap();
        assert_eq!(a_syn.0.seq, 100);
        assert!(a_syn.0.ctl.syn());

        // 3
        let mut peer_b = Tcb::open(PEER_B_ID, 300);
        assert_eq!(peer_b.state, State::SynSent);
        let b_syn = peer_b.outgoing.pop_back().unwrap();
        assert_eq!(b_syn.0.seq, 300);
        assert!(b_syn.0.ctl.syn());

        peer_a.segment_arrives(b_syn.0, b_syn.1).unwrap();
        assert_eq!(peer_a.state, State::SynReceived);

        // 4
        peer_b.segment_arrives(a_syn.0, a_syn.1).unwrap();
        assert_eq!(peer_b.state, State::SynReceived);

        // 5
        let a_syn_ack = peer_a.outgoing.pop_back().unwrap();
        assert!(a_syn_ack.0.ctl.syn());
        assert!(a_syn_ack.0.ctl.ack());
        assert_eq!(a_syn_ack.0.seq, 100);
        assert_eq!(a_syn_ack.0.ack, 301);

        // 6
        let b_syn_ack = peer_b.outgoing.pop_back().unwrap();
        assert!(b_syn_ack.0.ctl.syn());
        assert!(b_syn_ack.0.ctl.ack());
        assert_eq!(b_syn_ack.0.seq, 300);
        assert_eq!(b_syn_ack.0.ack, 101);

        peer_a.segment_arrives(b_syn_ack.0, b_syn_ack.1).unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 7
        peer_b.segment_arrives(a_syn_ack.0, a_syn_ack.1).unwrap();
        assert_eq!(peer_b.state, State::Established);
    }

    #[test]
    fn old_duplicate_syn() {
        // Based on 3.5 Figure 8:
        //
        //     TCP Peer A                                           TCP Peer B
        // 1.  CLOSED                                               LISTEN
        // 2.  SYN-SENT    --> <SEQ=100><CTL=SYN>               ...
        // 3.  (duplicate) ... <SEQ=90><CTL=SYN>                --> SYN-RECEIVED
        // 4.  SYN-SENT    <-- <SEQ=300><ACK=91><CTL=SYN,ACK>   <-- SYN-RECEIVED
        // 5.  SYN-SENT    --> <SEQ=91><CTL=RST>                --> LISTEN
        // 6.              ... <SEQ=100><CTL=SYN>               --> SYN-RECEIVED
        // 7.  ESTABLISHED <-- <SEQ=400><ACK=101><CTL=SYN,ACK>  <-- SYN-RECEIVED
        // 8.  ESTABLISHED --> <SEQ=101><ACK=401><CTL=ACK>      --> ESTABLISHED

        // 2
        let mut peer_a = Tcb::open(PEER_A_ID, 100);
        let peer_a_syn = peer_a.outgoing.pop_back().unwrap();
        assert!(peer_a_syn.0.ctl.syn());
        assert_eq!(peer_a_syn.0.seq, 100);

        // 3
        const GHOST_ID: ConnectionId = ConnectionId {
            local: Socket {
                address: Ipv4Address::new([123, 45, 67, 89]),
                port: 0xbabe,
            },
            remote: PEER_B_ID.local,
        };
        let mut ghost = Tcb::open(GHOST_ID, 90);
        let ghost_syn = ghost.outgoing.pop_back().unwrap();
        assert!(ghost_syn.0.ctl.syn());
        assert_eq!(ghost_syn.0.seq, 90);

        let mut peer_b = handle_listen(
            ghost_syn.0,
            ghost_syn.1,
            GHOST_ID.remote.address,
            GHOST_ID.local.address,
            300,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 4
        let peer_b_syn_ack = peer_b.outgoing.pop_back().unwrap();
        assert!(peer_b_syn_ack.0.ctl.syn());
        assert!(peer_b_syn_ack.0.ctl.ack());
        assert_eq!(peer_b_syn_ack.0.seq, 300);
        assert_eq!(peer_b_syn_ack.0.ack, 91);

        peer_a
            .segment_arrives(peer_b_syn_ack.0, peer_b_syn_ack.1)
            .unwrap();
        assert_eq!(peer_a.state, State::SynSent);

        // 5
        let peer_a_rst = peer_a.outgoing.pop_back().unwrap();
        assert!(peer_a_rst.0.ctl.rst());
        assert_eq!(peer_a_rst.0.seq, 91);

        let receive_result = peer_b.segment_arrives(peer_a_rst.0, peer_a_rst.1).unwrap();
        assert_eq!(receive_result, ReceiveResult::ReturnToListen);

        // 6
        let mut peer_b = handle_listen(
            peer_a_syn.0,
            peer_a_syn.1,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            400,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 7
        let peer_b_syn_ack = peer_b.outgoing.pop_back().unwrap();
        assert!(peer_b_syn_ack.0.ctl.syn());
        assert!(peer_b_syn_ack.0.ctl.ack());
        assert_eq!(peer_b_syn_ack.0.seq, 400);
        assert_eq!(peer_b_syn_ack.0.ack, 101);

        peer_a
            .segment_arrives(peer_b_syn_ack.0, peer_b_syn_ack.1)
            .unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 8
        let peer_a_ack = peer_a.outgoing.pop_back().unwrap();
        assert!(peer_a_ack.0.ctl.ack());
        assert_eq!(peer_a_ack.0.seq, 101);
        assert_eq!(peer_a_ack.0.ack, 401);
    }

    // TODO(hardint): Add tests for the exchanges in figures 9 through 11 about
    // half-open connections

    fn established_pair() -> (Tcb, Tcb) {
        let mut peer_a = Tcb::open(PEER_A_ID, 100);
        let peer_a_syn = peer_a.outgoing.pop_back().unwrap();
        let mut peer_b = handle_listen(
            peer_a_syn.0,
            peer_a_syn.1,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
        )
        .unwrap()
        .tcb()
        .unwrap();
        let peer_b_syn_ack = peer_b.outgoing.pop_back().unwrap();
        peer_a
            .segment_arrives(peer_b_syn_ack.0, peer_b_syn_ack.1)
            .unwrap();
        let peer_a_ack = peer_a.outgoing.pop_back().unwrap();
        peer_b.segment_arrives(peer_a_ack.0, peer_a_ack.1).unwrap();
        assert_eq!(peer_a.state, State::Established);
        assert_eq!(peer_b.state, State::Established);
        (peer_a, peer_b)
    }

    #[ignore]
    #[test]
    fn normal_close_sequence() {
        // This test implements the following exchange from 3.6, Figure 12:
        //
        //     TCP Peer A                                           TCP Peer B
        //
        // 1.  ESTABLISHED                                          ESTABLISHED
        //
        // 2.  (Close)
        //     FIN-WAIT-1  --> <SEQ=100><ACK=300><CTL=FIN,ACK>  --> CLOSE-WAIT
        //
        // 3.  FIN-WAIT-2  <-- <SEQ=300><ACK=101><CTL=ACK>      <-- CLOSE-WAIT
        //
        // 4.                                                       (Close)
        //     TIME-WAIT   <-- <SEQ=300><ACK=101><CTL=FIN,ACK>  <-- LAST-ACK
        //
        // 5.  TIME-WAIT   --> <SEQ=101><ACK=301><CTL=ACK>      --> CLOSED
        //
        // 6.  (2 MSL)
        //     CLOSED
        //
        // NOTE: MSL = Maximum Segment Lifetime

        // 1
        let (mut peer_a, mut peer_b) = established_pair();

        // 2
        peer_a.close();
        assert_eq!(peer_a.state, State::FinWait1);

        let peer_a_fin = peer_a.outgoing.pop_back().unwrap();
        assert!(peer_a_fin.0.ctl.fin());
        assert!(peer_a_fin.0.ctl.ack());
        assert_eq!(peer_a_fin.0.seq, 100);
        assert_eq!(peer_a_fin.0.ack, 300);

        peer_b.segment_arrives(peer_a_fin.0, peer_a_fin.1).unwrap();
        assert_eq!(peer_b.state, State::CloseWait);

        // 3
        let peer_b_ack = peer_b.outgoing.pop_back().unwrap();
        assert!(peer_b_ack.0.ctl.ack());
        assert_eq!(peer_b_ack.0.seq, 300);
        assert_eq!(peer_b_ack.0.ack, 101);

        peer_a.segment_arrives(peer_b_ack.0, peer_b_ack.1).unwrap();
        assert_eq!(peer_a.state, State::FinWait2);

        // 4
        peer_b.close();
        assert_eq!(peer_b.state, State::LastAck);

        let peer_b_fin = peer_b.outgoing.pop_back().unwrap();
        assert!(peer_b_fin.0.ctl.fin());
        assert!(peer_b_fin.0.ctl.ack());
        assert_eq!(peer_b_fin.0.seq, 300);
        assert_eq!(peer_b_fin.0.ack, 101);

        peer_a.segment_arrives(peer_b_fin.0, peer_b_fin.1).unwrap();
        assert_eq!(peer_a.state, State::TimeWait);

        // 5
        let peer_a_ack = peer_a.outgoing.pop_back().unwrap();
        assert!(peer_a_ack.0.ctl.ack());
        assert_eq!(peer_a_ack.0.seq, 101);
        assert_eq!(peer_a_ack.0.ack, 301);

        let receive_result = peer_b.segment_arrives(peer_a_ack.0, peer_a_ack.1).unwrap();
        assert_eq!(receive_result, ReceiveResult::FinalizeClose);

        let timeout = peer_a.advance_time(MSL.mul_f32(2.1));
        assert_eq!(timeout, AdvanceTimeResult::CloseConnection);
    }
}
