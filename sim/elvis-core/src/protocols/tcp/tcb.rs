use super::{
    tcp_parsing::{BuildHeaderError, TcpHeader, TcpHeaderBuilder},
    ConnectionId,
};
use crate::{
    protocols::{ipv4::Ipv4Address, utility::Socket},
    Message,
};
use std::collections::VecDeque;

// NOTE(hardint): Section numbers are base on RFC 9293, the updated TCP protocol
// specification

// NOTE(hardint): In the current implementation, we respond to with ACKs
// immediately instead of trying to piggyback them for efficiency. This can be revised once the implementation is working.

// TODO(hardint): Do actual window management
const RCV_WND: u16 = 4096;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tcb {
    id: ConnectionId,
    initiation: Initiation,
    state: State,
    snd: SendSequenceSpace,
    rcv: ReceiveSequenceSpace,
    outgoing: VecDeque<(TcpHeader, Message)>,
    incoming: VecDeque<(TcpHeader, Message)>,
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

    pub fn send(&mut self, _message: Message) -> Result<(), BuildHeaderError> {
        // 3.10.2
        todo!()
    }

    pub fn receive(&mut self) {
        // 3.10.3
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
        // 3.10.7 with special handling of CLOSED and LISTEN in freestanding
        // functions
        match self.state {
            State::SynSent => {
                // First:
                if seg.ctl.ack() {
                    if seg.ctl.rst() {
                        // Discard the segment
                        return Ok(ReceiveResult::DiscardSegment);
                    }

                    if mod_bounded(self.snd.nxt, Le, seg.ack, Leq, self.snd.iss) {
                        // Send a reset and discard the segment
                        self.enqueue_outgoing(
                            TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seg.ack)
                                .rst(),
                            [].into(),
                        )?;
                    }

                    if !mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                        return Ok(ReceiveResult::InvalidAck);
                    }
                }

                // Second:
                if seg.ctl.rst() {
                    if seg.seq == self.rcv.nxt {
                        return Ok(ReceiveResult::ConnectionReset);
                    } else {
                        return Err(ReceiveError::BlindReset);
                    };
                }

                // Third:
                // NOTE: Ignore security check

                // Fourth:
                if seg.ctl.syn() {
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

                if !seg.ctl.syn() && !seg.ctl.rst() {
                    return Ok(ReceiveResult::DiscardSegment);
                }
                // Otherwise, fall through for additional processing with the
                // sixth step of section 3.10.7.4
            }

            // Do First through Fifth, then break. The remaining steps are shared with SynSent.
            // 3.10.7.4
            State::SynReceived
            | State::Established
            | State::FinWait1
            | State::FinWait2
            | State::CloseWait
            | State::Closing
            | State::LastAck
            | State::TimeWait => {
                // TODO(hardint): Must process all queued segments before
                // sending any ACKs

                // Must process RST (and URG) of all incoming segments. Should
                // do this first so that early returns are acceptible. For the
                // same reason, ACKs should be processed early.

                // Second:
                if seg.ctl.rst() {
                    match self.state {
                        State::SynSent => unreachable!(),

                        State::SynReceived => match self.initiation {
                            Initiation::Listen => {
                                return Ok(ReceiveResult::CloseSilently);
                            }
                            Initiation::Open => {
                                return Ok(ReceiveResult::ConnectionRefused);
                            }
                        },

                        State::Established
                        | State::FinWait1
                        | State::FinWait2
                        | State::CloseWait => {
                            // TODO(hardint): Outstanding RECEIVEs and SENDs
                            // should receive reset responses.
                            return Ok(ReceiveResult::ConnectionReset);
                        }

                        State::Closing | State::LastAck | State::TimeWait => {
                            return Ok(ReceiveResult::CloseSilently);
                        }
                    }
                }

                // Third: Security check. Ignoring this part.

                // Fourth:
                if seg.ctl.syn() {
                    // NOTE(hardint): It's hard to tell from the spec if this is
                    // supposed to happen unconditionally or only if the SYN bit
                    // is set.
                    if self.state == State::SynReceived && self.initiation == Initiation::Listen {
                        return Ok(ReceiveResult::CloseSilently);
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

                // Fifth:
                if seg.ctl.ack() {
                    match self.state {
                        State::SynSent => unreachable!(),

                        State::SynReceived => {
                            if mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                                self.state = State::Established;
                                self.snd.wnd = seg.wnd;
                                self.snd.wl1 = seg.seq;
                                self.snd.wl2 = seg.ack;
                            } else {
                                self.enqueue_outgoing(
                                    self.header_builder(seg.ack).rst(),
                                    [].into(),
                                )?;
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
            }
        }

        // NOTE(hardint): Continuing with Other States processing, 3.10.7.4

        // Sixth: Check the URG bit. Ignoring this part.

        // First: Doing this late because "special allowance should be made to
        // accept valid ACKs, URGs, and RSTs"
        if !self.is_segment_acceptable(message.len() as u32, seg.seq) {
            self.enqueue_outgoing(
                self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                [].into(),
            )?;
            return Ok(ReceiveResult::DiscardSegment);
        }

        // Seventh: Process the segment text
        match self.state {
            State::Established | State::FinWait1 | State::FinWait2 => {
                // TODO(hardint): Once in the ESTABLISHED state, it is possible
                // to deliver segment data to user RECEIVE buffers. Data from
                // segments can be moved into buffers until either the buffer is
                // full or the segment is empty. If the segment empties and
                // carries a PUSH flag, then the user is informed, when the
                // buffer is returned, that a PUSH has been received. When the
                // TCP endpoint takes responsibility for delivering the data to
                // the user, it must also acknowledge the receipt of the data.
                // Once the TCP endpoint takes responsibility for the data, it
                // advances RCV.NXT over the data accepted, and adjusts RCV.WND
                // as appropriate to the current buffer availability. The total
                // of RCV.NXT and RCV.WND should not be reduced. A TCP
                // implementation MAY send an ACK segment acknowledging RCV.NXT
                // when a valid segment arrives that is in the window but not at
                // the left window edge (MAY-13). Please note the window
                // management suggestions in Section 3.8. Send an acknowledgment
                // of the form: <SEQ=SND.NXT><ACK=RCV.NXT><CTL=ACK> This
                // acknowledgment should be piggybacked on a segment being
                // transmitted if possible without incurring undue delay.
            }

            State::SynSent
            | State::SynReceived
            | State::CloseWait
            | State::Closing
            | State::LastAck
            | State::TimeWait => {}
        }

        // Eighth:
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

    pub fn advance_time(_ms: u64) {
        // TODO(hardint): See 3.10.8 for timeout handling
        todo!()
    }

    fn is_segment_acceptable(&self, seg_len: u32, seq: u32) -> bool {
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

pub enum ReceiveResult {
    Success,
    DiscardSegment,
    InvalidAck,
    UnacceptableSegment,
    CloseSilently,
    ConnectionReset,
    ConnectionRefused,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("{0}")]
    Header(#[from] BuildHeaderError),
    #[error("SEG.RST and RCV.NXT != SEG.SEQ")]
    BlindReset,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Initiation {
    Listen,
    Open,
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
        tcb.incoming.push_back((seg, message));

        Some(ListenResult::Tcb(tcb))
    } else {
        // Fourth:
        // Any other control or data-bearing segment should be discarded
        None
    }
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

use ModCmp::*;

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

    #[test]
    fn section_3_5_fig_6() {
        // 1
        // Peer A: CLOSED
        // Peer B: LISTEN

        let peer_a_id = ConnectionId {
            local: Socket {
                address: 0.into(),
                port: 0xcafe,
            },
            remote: Socket {
                address: 1.into(),
                port: 0xdead,
            },
        };
        let peer_b_id = peer_a_id.reverse();

        // 2
        let mut peer_a = Tcb::open(peer_a_id, 100);
        assert_eq!(peer_a.state, State::SynSent);
        let (header, message) = peer_a.outgoing.pop_back().unwrap();
        assert_eq!(header.seq, 100);
        assert!(header.ctl.syn());

        let mut peer_b = handle_listen(
            header,
            message,
            peer_b_id.local.address,
            peer_b_id.remote.address,
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
}
