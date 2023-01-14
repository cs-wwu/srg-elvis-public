use super::{
    tcp_parsing::{BuildHeaderError, TcpHeader, TcpHeaderBuilder},
    ConnectionId,
};
use crate::{
    protocols::{ipv4::Ipv4Address, utility::Socket},
    Message,
};
use std::collections::VecDeque;

// TODO(hardint): Do more precise window management
const RCV_WND: u16 = u16::MAX;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tcb {
    id: ConnectionId,
    initiation: Initiation,
    state: State,
    snd: SendSequenceSpace,
    rcv: ReceiveSequenceSpace,
    queue: VecDeque<(TcpHeader, Message)>,
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
            queue: Default::default(),
        }
    }

    pub fn open(id: ConnectionId, iss: u32) -> Self {
        // see 3.10.1
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
        tcb.enqueue(tcb.header_builder(iss).syn(), [].into())
            .unwrap();
        tcb
    }

    pub fn send(&mut self, _message: Message) -> Result<(), BuildHeaderError> {
        todo!()
    }

    fn header_builder(&self, seq: u32) -> TcpHeaderBuilder {
        TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seq)
    }

    fn enqueue(
        &mut self,
        header_builder: TcpHeaderBuilder,
        message: Message,
    ) -> Result<(), BuildHeaderError> {
        let header = header_builder.build(
            self.id.local.address,
            self.id.remote.address,
            message.iter(),
        )?;
        self.queue.push_back((header, message));
        Ok(())
    }

    pub fn receive(
        &mut self,
        seg: TcpHeader,
        message: Message,
    ) -> Result<ReceiveResult, ReceiveError> {
        match self.state {
            State::SynSent => {
                // First:
                if seg.ctl.ack() {
                    if seg.ctl.rst() {
                        // Discard the segment
                        return Ok(ReceiveResult::DiscardSegment);
                    }

                    if is_between_wrapped(self.snd.nxt, seg.ack, self.snd.iss + 1) {
                        // Send a reset and discard the segment
                        self.enqueue(
                            TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seg.ack)
                                .rst(),
                            [].into(),
                        )?;
                    }

                    if !is_between_wrapped(self.snd.una, seg.ack, self.snd.nxt) {
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
                    self.snd.una = seg.ack;

                    if self.snd.una > self.snd.iss {
                        self.state = State::Established;
                        self.enqueue(
                            self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                            [].into(),
                        )?;
                    } else {
                        self.state = State::SynReceived;
                        self.enqueue(
                            self.header_builder(self.snd.iss).syn().ack(self.rcv.nxt),
                            [].into(),
                        )?;
                        self.snd.wnd = seg.wnd;
                        self.snd.wl1 = seg.seq;
                        self.snd.wl2 = seg.ack;
                        // TODO(hardint): Queue other controls or text for
                        // processing in Established state
                    }

                    return Ok(ReceiveResult::Success);
                }

                return Ok(ReceiveResult::DiscardSegment);
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
                // Segments are processed in sequence. Initial tests on arrival
                // are used to discard old duplicates, but further processing is
                // done in SEG.SEQ order. If a segment's contents straddle the
                // boundary between old and new, only the new parts are
                // processed.

                // TODO(hardint): Must process all queued segments before
                // sending any ACKs

                // Must process RST (and URG) of all incoming segments. Should
                // do this first so that early returns are acceptible. For the
                // same reason, ACKs should be processed early.

                // Second:
                if seg.ctl.rst() {
                    match self.state {
                        // We already handled this state
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
            }
        }

        Ok(ReceiveResult::Success)
    }

    fn is_segment_acceptible(&self, seg_len: u32, seq: u32) -> bool {
        // Test segment acceptability. See Table 6.
        if seg_len == 0 {
            // TODO(hardint): Unreachable right now, but when window
            // management is added, this will be more important
            if RCV_WND == 0 {
                if seq == self.rcv.nxt {
                    // Okay!
                } else {
                    return false;
                }
            } else {
                if self.is_seq_in_window(seq) {
                    // Okay!
                } else {
                    return false;
                }
            }
        } else {
            // TODO(hardint): Unreachable right now, but when window
            // management is added, this will be more important
            if RCV_WND == 0 {
                // When the receive window is zero, only ACKs are acceptible.
                return false;
            } else {
                if self.is_seq_in_window(seq)
                    || is_between_wrapped(
                        self.rcv.nxt - 1,
                        seq + seg_len - 1,
                        self.rcv.nxt + RCV_WND as u32,
                    )
                {
                    // Okay!
                } else {
                    return false;
                }
            }
        }
        true
    }

    fn is_seq_in_window(&self, seq: u32) -> bool {
        is_between_wrapped(self.rcv.nxt - 1, seq, self.rcv.nxt + RCV_WND as u32)
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
    seg: TcpHeader,
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
            .map(|header| ListenResult::Response(header))
    } else if seg.ctl.syn() {
        // Third:
        // Open the connection

        // NOTE: Ignore security check for simplicity

        // TODO(hardint): Any other control or text should be queued for
        // processing later
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
        tcb.enqueue(tcb.header_builder(iss), [].into()).ok()?;
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
        let (header, _message) = peer_a.queue.pop_back().unwrap();
        assert_eq!(header.seq, 100);
        assert!(header.ctl.syn());

        let mut peer_b = handle_listen(
            header,
            peer_b_id.local.address,
            peer_b_id.remote.address,
            300,
        )
        .unwrap()
        .tcb()
        .unwrap();
        assert_eq!(peer_b.state, State::SynReceived);

        // 3
        let (header, message) = peer_b.queue.pop_back().unwrap();
        assert_eq!(header.seq, 300);
        assert_eq!(header.ack, 101);
        assert!(header.ctl.syn());
        assert!(header.ctl.ack());

        peer_a.receive(header, message).unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 4
        let (header, message) = peer_a.queue.pop_back().unwrap();
        assert_eq!(header.seq, 101);
        assert_eq!(header.ack, 301);
        assert!(header.ctl.ack());

        peer_b.receive(header, message).unwrap();
        assert_eq!(peer_b.state, State::Established);

        // 5
        peer_a.send(Message::new("Hello!")).unwrap();
        let (header, message) = peer_a.queue.pop_back().unwrap();
        assert_eq!(header.seq, 101);
        assert_eq!(header.ack, 301);
        assert!(header.ctl.ack());
        assert_eq!(message.len(), 6);

        peer_b.receive(header, message).unwrap();
        assert_eq!(peer_b.state, State::Established);
    }
}
