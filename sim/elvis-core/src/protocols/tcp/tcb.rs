use super::{
    tcp_parsing::{TcpHeader, TcpHeaderBuilder},
    ConnectionId,
};
use crate::{
    network::Mtu,
    protocols::{ipv4::Ipv4Address, utility::Socket},
    Message,
};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
    time::Duration,
};

// TODO(hardint): Move acknowledgment queuing to the front so they get delivered first

// NOTE(hardint): Section numbers are base on RFC 9293, the updated TCP protocol
// specification

// NOTE(hardint): In the current implementation, we respond to with ACKs
// immediately instead of trying to piggyback them for efficiency. This can be revised once the implementation is working.

// TODO(hardint): Choose a more realistic value
/// The maximum segment lifetime on the Internet
const MSL: Duration = Duration::from_secs(1);

// TODO(hardint): Choose a better value
/// The time that may pass before packets are retransmitted
const RETRANSMISSION_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct Tcb {
    id: ConnectionId,
    mtu: Mtu,
    initiation: Initiation,
    state: State,
    snd: SendSequenceSpace,
    rcv: ReceiveSequenceSpace,
    outgoing: Outgoing,
    incoming: BinaryHeap<Incoming>,
    received_text: VecDeque<Message>,
    retransmission_timeout: Duration,
    time_wait_timeout: Option<Duration>,
}

impl Tcb {
    fn new(
        id: ConnectionId,
        mtu: Mtu,
        initiation: Initiation,
        state: State,
        snd: SendSequenceSpace,
        rcv: ReceiveSequenceSpace,
    ) -> Self {
        Self {
            id,
            mtu,
            initiation,
            state,
            snd,
            rcv,
            outgoing: Default::default(),
            incoming: Default::default(),
            received_text: Default::default(),
            retransmission_timeout: RETRANSMISSION_TIMEOUT,
            time_wait_timeout: None,
        }
    }

    pub fn open(id: ConnectionId, iss: u32, mtu: Mtu) -> Self {
        // 3.10.1 Specifically for the case of an active open. Handling for
        // packets in a passive open LISTEN state is provided in a freestanding
        // function.
        let mut tcb = Self::new(
            id,
            mtu,
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
        tcb.enqueue(tcb.header_builder(iss).syn());
        tcb
    }

    pub fn advance_time(&mut self, delta_time: Duration) -> AdvanceTimeResult {
        if delta_time > self.retransmission_timeout {
            self.retransmission_timeout = RETRANSMISSION_TIMEOUT;
            for mut transmit in self.outgoing.retransmit.iter_mut() {
                transmit.needs_transmit = true;
            }
        } else {
            self.retransmission_timeout -= delta_time;
        }

        if let Some(time_wait) = self.time_wait_timeout {
            if delta_time > time_wait {
                return AdvanceTimeResult::CloseConnection;
            }
            self.time_wait_timeout = Some(time_wait - delta_time);
        }

        AdvanceTimeResult::Ignore
    }

    pub fn send(&mut self, message: Message) {
        // 3.10.2 (Not compliant, doing things differently. We don't have a
        // retransmission queue.)
        self.outgoing.text.push_back(message);
    }

    pub fn receive(&mut self) -> Vec<u8> {
        // 3.10.3

        // TODO(hardint): Use receive buffer size instead of just taking
        // everything
        let bytes = self
            .received_text
            .iter()
            .map(|message| message.len())
            .sum();
        consume_text(&mut self.received_text, bytes)
    }

    pub fn close(&mut self) -> CloseResult {
        // 3.10.4
        match self.state {
            State::SynSent => CloseResult::CloseConnection,

            State::SynReceived | State::Established => {
                // TODO(hardint): Should only do this if there is no pending
                // data to send, but I don't have a good mechanism for queuing
                // things like this for later processing once we reach
                // ESTABLISHED as the spec describes, so this is how it is for
                // now.

                // TODO(hardint): Is this the correct sequence number for a FIN
                // segment?
                self.enqueue(self.header_builder(self.snd.nxt + 1));
                self.state = State::FinWait1;
                CloseResult::Ok
            }

            State::CloseWait => {
                self.enqueue(self.header_builder(self.snd.nxt + 1));
                self.state = State::LastAck;
                CloseResult::Ok
            }

            State::FinWait1
            | State::FinWait2
            | State::Closing
            | State::LastAck
            | State::TimeWait => CloseResult::ConnectionClosing,
        }
    }

    // Should delete the TCB after this call once the final RST segment is
    // delivered, if present.
    pub fn abort(&mut self) {
        // 3.10.5
        self.outgoing = Default::default();
        if self.state == State::CloseWait {
            self.enqueue(self.header_builder(self.snd.nxt).rst());
        }
    }

    pub fn status(&self) -> State {
        // 3.10.6
        self.state
    }

    pub fn segments(&mut self) -> Vec<Segment> {
        let mut out: Vec<_> = std::mem::take(&mut self.outgoing.oneshot)
            .into_iter()
            .map(|header| Segment::new(header, [].into()))
            .collect();

        match self.state {
            State::SynSent | State::SynReceived | State::Established | State::CloseWait => {}
            State::FinWait1
            | State::FinWait2
            | State::Closing
            | State::LastAck
            | State::TimeWait => return out,
        }

        // TODO(hardint): This could be incorrect for when optional
        // headers are used. It also is not as efficient as possible.
        const SPACE_FOR_HEADERS: u32 = 50;
        let max_segment_length = (self.mtu - SPACE_FOR_HEADERS) as usize;
        let mut queued_bytes = self.outgoing.queued_bytes();
        loop {
            let max_bytes = self.snd.wnd as usize - queued_bytes;
            let text = consume_text(&mut self.outgoing.text, max_segment_length.min(max_bytes));
            if text.is_empty() {
                break;
            }
            queued_bytes += text.len();
            let header = self
                .header_builder(self.snd.nxt)
                .ack(self.rcv.nxt)
                .wnd(self.rcv.wnd)
                .build(
                    self.id.local.address,
                    self.id.remote.address,
                    text.iter().cloned(),
                )
                .expect("Unexpectedly large MTU and message");
            self.snd.nxt = self.snd.nxt.wrapping_add(text.len() as u32);
            self.outgoing
                .retransmit
                .push_back(Transmit::new(Segment::new(header, Message::new(text))));
        }

        for transmit in self.outgoing.retransmit.iter_mut() {
            if transmit.needs_transmit {
                out.push(transmit.segment.clone());
            }
            transmit.needs_transmit = false;
        }

        out
    }

    fn remove_acked_from_retransmission(&mut self) {
        let mut i = 0;
        while let Some(transmit) = self.outgoing.retransmit.get(i) {
            let seq = transmit.segment.header.seq;
            let seg_len = transmit.segment.seg_len() as u32;
            if mod_le(self.snd.una, seq + seg_len) {
                i += 1;
            } else {
                self.outgoing.retransmit.remove(i);
            }
        }
    }

    pub fn segment_arrives(&mut self, segment: Segment) -> SegmentArrivesResult {
        self.incoming.push(Incoming::new(segment));
        while let Some(segment) = self.incoming.peek() {
            if self.state != State::SynSent && mod_ge(segment.0.header.seq, self.rcv.nxt) {
                // If this segment is past the next byte we want to receive, it
                // arrived out of order and we haven't received the earlier
                // bytes we need to proceed.
                break;
            }
            let segment = self.incoming.pop().unwrap().into_inner();
            let receive_result = self.process_segment(segment);
            match receive_result {
                ProcessSegmentResult::Success
                | ProcessSegmentResult::DiscardSegment
                | ProcessSegmentResult::InvalidAck => {}
                ProcessSegmentResult::ReturnToListen
                | ProcessSegmentResult::ConnectionReset
                | ProcessSegmentResult::ConnectionRefused
                | ProcessSegmentResult::FinalizeClose
                | ProcessSegmentResult::BlindReset => {
                    return SegmentArrivesResult::Close;
                }
            }
        }
        SegmentArrivesResult::Ok
        // TODO(hardint): Aggregate ACK segments
    }

    fn process_segment(&mut self, segment: Segment) -> ProcessSegmentResult {
        let (seg, mut text) = segment.into_inner();

        match self.state {
            // Sequence number checks don't apply for LISTEN, SYN-SENT, or CLOSING
            State::SynSent | State::Closing => {}
            _ => {
                if !self.is_seq_ok(text.len() as u32, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                    return ProcessSegmentResult::DiscardSegment;
                }
            }
        }

        if seg.ctl.ack() {
            match self.state {
                State::SynSent => {
                    if mod_bounded(self.snd.nxt, Le, seg.ack, Leq, self.snd.iss) {
                        if seg.ctl.rst() {
                            // Discard the segment
                            return ProcessSegmentResult::DiscardSegment;
                        } else {
                            self.enqueue(self.header_builder(seg.ack).rst());
                            return ProcessSegmentResult::InvalidAck;
                        }
                    }

                    if mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                        // Valid acknowledgment
                        if seg.ctl.syn() {
                            // The spec doesn't specifically describe what to do for
                            // on okay ACK in SYN-SENT, but I think this is what is
                            // supposed to happen
                            self.snd.una = seg.ack;
                            self.remove_acked_from_retransmission();
                        } else {
                            // What has been happening is that the listen side
                            // of the connection will generate a challenge ACK
                            // in response to receiving a duplicate SYN. That
                            // comes back to us first and we update SND.UNA as
                            // above. Later, when the SYN ACK arrives with the
                            // same acknowledgment, SND.UNA==SEG.ACK causes the
                            // acknowledgment to be rejected and the connection
                            // is reset. Therefore, we only proceed to process
                            // the ACK segment if it comes along with a SYN.
                        }
                    } else {
                        // Same ACK twice causes this failure
                        self.enqueue(self.header_builder(seg.ack).rst());
                        return ProcessSegmentResult::InvalidAck;
                    }
                }

                State::SynReceived => {
                    if mod_bounded(self.snd.una, Le, seg.ack, Leq, self.snd.nxt) {
                        self.state = State::Established;
                        self.snd.wnd = seg.wnd;
                        self.snd.wl1 = seg.seq;
                        self.snd.wl2 = seg.ack;
                        match self.ack_established_processing(&seg) {
                            ProcessSegmentResult::Success => {}
                            other => return other,
                        }
                    } else {
                        self.enqueue(self.header_builder(seg.ack).rst());
                    }
                }

                // ESTABLISHED processing:
                State::Established
                | State::FinWait1
                | State::FinWait2
                | State::CloseWait
                | State::Closing => match self.ack_established_processing(&seg) {
                    ProcessSegmentResult::Success => {}
                    other => return other,
                },

                State::LastAck => {
                    // TODO(hardint): If our FIN is now acknowledged,
                    // close the TCB
                    todo!()
                }

                State::TimeWait => {
                    // TODO(hardint): The only thing that can arrive is
                    // a retransmission of the remote FIN. Acknowledge
                    // it and restart the MSL 2 timeout.
                    todo!()
                }
            }
        }

        if seg.ctl.rst() {
            match self.state {
                State::SynSent => {
                    if seg.seq == self.rcv.nxt {
                        return ProcessSegmentResult::ConnectionReset;
                    } else {
                        return ProcessSegmentResult::BlindReset;
                    };
                }

                State::SynReceived => match self.initiation {
                    Initiation::Listen => {
                        return ProcessSegmentResult::ReturnToListen;
                    }
                    Initiation::Open => {
                        return ProcessSegmentResult::ConnectionRefused;
                    }
                },

                State::Established | State::FinWait1 | State::FinWait2 | State::CloseWait => {
                    // TODO(hardint): Outstanding RECEIVEs and SENDs
                    // should receive reset responses.
                    return ProcessSegmentResult::ConnectionReset;
                }

                State::Closing | State::LastAck | State::TimeWait => {
                    return ProcessSegmentResult::FinalizeClose;
                }
            }
        }

        if seg.ctl.syn() {
            match self.state {
                State::SynSent => {
                    self.rcv.irs = seg.seq;
                    self.rcv.nxt = seg.seq + 1;

                    // Already did ACK processing

                    self.snd.wnd = seg.wnd;
                    self.snd.wl1 = seg.seq;
                    self.snd.wl2 = seg.ack;
                    if mod_ge(self.snd.una, self.snd.iss) {
                        self.state = State::Established;
                        self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                    } else {
                        self.state = State::SynReceived;
                        self.enqueue(self.header_builder(self.snd.iss).syn().ack(self.rcv.nxt));
                        return ProcessSegmentResult::Success;
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
                    // We are ignoring some of the spec's guidance around
                    // closing the connection if we get a SYN in an established
                    // state. It seems to create a lot of failed connections due
                    // to delayed SYN packets. We do a subset of what the spec
                    // suggests and just send a challenge ACK, which is
                    // important for the case where a peer generates an ACK in
                    // response to a SYN ACK and the ACK gets lost in
                    // transmission. The challenge ACK regenerates the lost ACK
                    // segment.
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                    return ProcessSegmentResult::DiscardSegment;
                }
            }
        }

        // Queue the segment text for processing
        if !text.is_empty() {
            match self.state {
                State::Established
                | State::SynSent
                | State::SynReceived
                | State::FinWait1
                | State::FinWait2 => {
                    // If we got here, we already know that SEQ > RCV.NXT
                    // Should also be in the window, but let's check:
                    assert!(
                        self.is_in_rcv_window(seg.seq)
                            || self.is_in_rcv_window(seg.seq + text.len() as u32)
                    );
                    let already_received = self.rcv.nxt - seg.seq; // Works with modulus
                    let seg_len = text.len() as u32 + seg.ctl.syn() as u32 + seg.ctl.fin() as u32;
                    let unreceived = seg_len - already_received;
                    let accept = unreceived.min(self.rcv.wnd as u32);
                    self.rcv.nxt += accept;
                    text.slice(already_received as usize..(already_received + accept) as usize);
                    self.received_text.push_back(text);
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                    // TODO(hardint): Aggregate and piggyback ACK segments
                }

                State::CloseWait | State::Closing | State::LastAck | State::TimeWait => {
                    // Ignore the segment text
                }
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
                    todo!()
                }

                State::FinWait2 => {
                    self.state = State::TimeWait;
                    // Start the time-wait timer, turn off the other timers.
                    self.time_wait_timeout = Some(2 * MSL);
                    self.retransmission_timeout = RETRANSMISSION_TIMEOUT;
                }

                State::TimeWait => {
                    // Restart the 2 MSL time-wait timeout.
                    self.time_wait_timeout = Some(2 * MSL);
                }
            }
        }

        ProcessSegmentResult::Success
    }

    fn ack_established_processing(&mut self, seg: &TcpHeader) -> ProcessSegmentResult {
        if mod_leq(seg.ack, self.snd.una) {
            // Ignore duplicate ACK
            return ProcessSegmentResult::Success;
        } else if mod_ge(seg.ack, self.snd.nxt) {
            // ACKs something not yet sent
            self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
            return ProcessSegmentResult::DiscardSegment;
        } else {
            // Valid ACK
            self.snd.una = seg.ack;
            self.remove_acked_from_retransmission();
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
                todo!()
            }

            State::FinWait2 => {
                // TODO(hardint): If the retransmission
                // queue is empty, acknowledge the user's
                // CLOSE
                todo!()
            }

            State::Closing => {
                // TODO(hardint): If the ACK acknowledges
                // our FIN, enter TIME-WAIT.
                todo!()
            }

            _ => {}
        }

        ProcessSegmentResult::Success
    }

    fn header_builder(&self, seq: u32) -> TcpHeaderBuilder {
        TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seq)
    }

    fn enqueue(&mut self, header_builder: TcpHeaderBuilder) {
        let header = header_builder
            .wnd(self.rcv.wnd)
            .build(
                self.id.local.address,
                self.id.remote.address,
                [].into_iter(),
            )
            // Okay for short segments
            .unwrap();
        if header.ctl.syn() || header.ctl.fin() {
            self.outgoing
                .retransmit
                .push_back(Transmit::new(Segment::new(header, [].into())));
        } else {
            self.outgoing.oneshot.push(header);
        }
    }

    fn is_ack_ok(&self, ack: u32) -> bool {
        mod_bounded(self.snd.una, Le, ack, Leq, self.snd.nxt)
    }

    fn is_seq_ok(&self, data_len: u32, seq: u32, syn: bool, fin: bool) -> bool {
        let seg_len = data_len + fin as u32 + syn as u32;
        // Test segment acceptability. See Table 6.
        if seg_len == 0 {
            if self.rcv.wnd == 0 {
                mod_bounded(self.rcv.nxt - 1, Leq, seq, Leq, self.rcv.nxt)
            } else {
                self.is_in_rcv_window(seq)
            }
        } else if self.rcv.wnd == 0 {
            // When the receive window is zero, only ACKs are acceptible.
            false
        } else {
            self.is_in_rcv_window(seq) || self.is_in_rcv_window(seq + seg_len - 1)
        }
    }

    fn is_in_rcv_window(&self, n: u32) -> bool {
        // NOTE(hardint): The original design for sequence number validation
        // fails under certain situations, such as simultaneous open. Appendix
        // A.2 links to a revision to sequence number validation that we employ:
        // https://datatracker.ietf.org/doc/html/draft-gont-tcpm-tcp-seq-validation-04
        // See page 10
        mod_bounded(
            self.rcv.nxt - 1,
            Leq,
            n,
            Le,
            self.rcv.nxt + self.rcv.wnd as u32,
        )
    }

    fn is_in_snd_window(&self, n: u32) -> bool {
        mod_bounded(self.snd.nxt, Leq, n, Le, self.snd.nxt + self.snd.wnd as u32)
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
    segment: Segment,
    local: Ipv4Address,
    remote: Ipv4Address,
    iss: u32,
    mtu: Mtu,
) -> Option<ListenResult> {
    let (mut seg, message) = segment.into_inner();
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
            mtu,
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
                ..Default::default()
            },
        );
        tcb.enqueue(tcb.header_builder(iss).syn().ack(tcb.rcv.nxt));

        // Processing of SYN and ACK should not be repeated.
        seg.ctl.set_syn(false);
        seg.ctl.set_ack(false);
        tcb.incoming.push(Incoming::new(Segment::new(seg, message)));

        Some(ListenResult::Tcb(tcb))
    } else {
        // Fourth:
        // Any other control or data-bearing segment should be discarded
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
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
    /// Oldest unacknowledged sequence number
    una: u32,
    /// Next sequence number to be sent
    nxt: u32,
    /// The size of the remote TCP's window
    wnd: u16,
    /// Segment sequence number used for last window update
    wl1: u32,
    /// Segment acknowledgment number used for last window update
    wl2: u32,
    /// Initial send sequence number
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct ReceiveSequenceSpace {
    /// Initial receive sequence number
    irs: u32,
    /// Next sequence number expected on an incoming segment, and is the
    /// left or lower edge of the receive window
    nxt: u32,
    /// The number of bytes we can buffer from the remote TCP
    wnd: u16,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
enum ProcessSegmentResult {
    Success,
    DiscardSegment,
    InvalidAck,
    ReturnToListen,
    ConnectionReset,
    ConnectionRefused,
    FinalizeClose,
    BlindReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentArrivesResult {
    Ok,
    Close,
}

pub enum SendResult {
    Ok,
    ClosingConnection,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub header: TcpHeader,
    pub text: Message,
}

impl Segment {
    pub fn new(seg: TcpHeader, message: Message) -> Self {
        Self {
            header: seg,
            text: message,
        }
    }

    /// The length of the segment data, including any control bits
    pub fn seg_len(&self) -> usize {
        self.text.len() + self.header.ctl.syn() as usize + self.header.ctl.fin() as usize
    }

    pub fn into_inner(self) -> (TcpHeader, Message) {
        (self.header, self.text)
    }
}

#[derive(Debug, Clone)]
struct Incoming(Segment);

impl Incoming {
    pub fn new(segment: Segment) -> Self {
        Self(segment)
    }

    pub fn into_inner(self) -> Segment {
        self.0
    }
}

impl PartialEq for Incoming {
    fn eq(&self, other: &Self) -> bool {
        self.0.header.seq == other.0.header.seq
    }
}

impl Eq for Incoming {}

impl PartialOrd for Incoming {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Incoming {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0.header.seq == other.0.header.seq {
            Ordering::Equal
        } else if mod_le(self.0.header.seq, other.0.header.seq) {
            // Reversing the order so the the priority queue handles messages
            // starting from lower sequence numbers
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Outgoing {
    /// Bytes already gobbled from the front of the first message in `text`.
    text: VecDeque<Message>,
    retransmit: VecDeque<Transmit>,
    oneshot: Vec<TcpHeader>,
}

impl Outgoing {
    pub fn queued_bytes(&self) -> usize {
        self.retransmit
            .iter()
            .map(|transmit| transmit.segment.text.len())
            .sum()
    }
}

fn consume_text(queue: &mut VecDeque<Message>, bytes: usize) -> Vec<u8> {
    let mut out = vec![];
    while let Some(mut text) = queue.pop_front() {
        if text.len() <= bytes {
            out.extend(text.iter());
        } else {
            out.extend(text.iter().take(bytes));
            text.slice(bytes..);
            queue.push_front(text);
            break;
        }
    }
    out
}

#[derive(Debug, Clone)]
struct Transmit {
    segment: Segment,
    needs_transmit: bool,
}

impl Transmit {
    pub fn new(segment: Segment) -> Self {
        Self {
            segment,
            needs_transmit: true,
        }
    }
}

pub enum CloseResult {
    Ok,
    CloseConnection,
    ConnectionClosing,
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
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        assert_eq!(peer_a.state, State::SynSent);
        let peer_a_syn = peer_a.segments().remove(0);
        assert_eq!(peer_a_syn.header.seq, 100);
        assert!(peer_a_syn.header.ctl.syn());

        let mut peer_b = handle_listen(
            peer_a_syn,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();
        assert_eq!(peer_b.state, State::SynReceived);

        // 3
        let peer_b_syn_ack = peer_b.segments().remove(0);
        assert_eq!(peer_b_syn_ack.header.seq, 300);
        assert_eq!(peer_b_syn_ack.header.ack, 101);
        assert!(peer_b_syn_ack.header.ctl.syn());
        assert!(peer_b_syn_ack.header.ctl.ack());

        peer_a.segment_arrives(peer_b_syn_ack);
        assert_eq!(peer_a.state, State::Established);

        // 4
        let peer_a_ack = peer_a.segments().remove(0);
        assert_eq!(peer_a_ack.header.seq, 101);
        assert_eq!(peer_a_ack.header.ack, 301);
        assert!(peer_a_ack.header.ctl.ack());

        peer_b.segment_arrives(peer_a_ack);
        assert_eq!(peer_b.state, State::Established);

        // 5 TODO(hardint): Needs data segment transmission to work
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
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        assert_eq!(peer_a.state, State::SynSent);
        let a_syn = peer_a.segments().remove(0);
        assert_eq!(a_syn.header.seq, 100);
        assert!(a_syn.header.ctl.syn());

        // 3
        let mut peer_b = Tcb::open(PEER_B_ID, 300, 1500);
        assert_eq!(peer_b.state, State::SynSent);
        let b_syn = peer_b.segments().remove(0);
        assert_eq!(b_syn.header.seq, 300);
        assert!(b_syn.header.ctl.syn());

        peer_a.segment_arrives(b_syn);
        assert_eq!(peer_a.state, State::SynReceived);

        // 4
        peer_b.segment_arrives(a_syn);
        assert_eq!(peer_b.state, State::SynReceived);

        // 5
        let a_syn_ack = peer_a.segments().remove(0);
        assert!(a_syn_ack.header.ctl.syn());
        assert!(a_syn_ack.header.ctl.ack());
        assert_eq!(a_syn_ack.header.seq, 100);
        assert_eq!(a_syn_ack.header.ack, 301);

        // 6
        let b_syn_ack = peer_b.segments().remove(0);
        assert!(b_syn_ack.header.ctl.syn());
        assert!(b_syn_ack.header.ctl.ack());
        assert_eq!(b_syn_ack.header.seq, 300);
        assert_eq!(b_syn_ack.header.ack, 101);

        peer_a.segment_arrives(b_syn_ack);
        assert_eq!(peer_a.state, State::Established);

        // 7
        peer_b.segment_arrives(a_syn_ack);
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
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        let peer_a_syn = peer_a.segments().remove(0);
        assert!(peer_a_syn.header.ctl.syn());
        assert_eq!(peer_a_syn.header.seq, 100);

        // 3
        const GHOST_ID: ConnectionId = ConnectionId {
            local: Socket {
                address: Ipv4Address::new([123, 45, 67, 89]),
                port: 0xbabe,
            },
            remote: PEER_B_ID.local,
        };
        let mut ghost = Tcb::open(GHOST_ID, 90, 1500);
        let ghost_syn = ghost.segments().remove(0);
        assert!(ghost_syn.header.ctl.syn());
        assert_eq!(ghost_syn.header.seq, 90);

        let mut peer_b = handle_listen(
            ghost_syn,
            GHOST_ID.remote.address,
            GHOST_ID.local.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 4
        let peer_b_syn_ack = peer_b.segments().remove(0);
        assert!(peer_b_syn_ack.header.ctl.syn());
        assert!(peer_b_syn_ack.header.ctl.ack());
        assert_eq!(peer_b_syn_ack.header.seq, 300);
        assert_eq!(peer_b_syn_ack.header.ack, 91);

        peer_a.segment_arrives(peer_b_syn_ack);
        assert_eq!(peer_a.state, State::SynSent);

        // 5
        let peer_a_rst = peer_a.segments().remove(0);
        assert!(peer_a_rst.header.ctl.rst());
        assert_eq!(peer_a_rst.header.seq, 91);

        let receive_result = peer_b.segment_arrives(peer_a_rst);
        assert_eq!(receive_result, SegmentArrivesResult::Close);

        // 6
        let mut peer_b = handle_listen(
            peer_a_syn,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            400,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 7
        let peer_b_syn_ack = peer_b.segments().remove(0);
        assert!(peer_b_syn_ack.header.ctl.syn());
        assert!(peer_b_syn_ack.header.ctl.ack());
        assert_eq!(peer_b_syn_ack.header.seq, 400);
        assert_eq!(peer_b_syn_ack.header.ack, 101);

        peer_a.segment_arrives(peer_b_syn_ack);
        assert_eq!(peer_a.state, State::Established);

        // 8
        let peer_a_ack = peer_a.segments().remove(0);
        assert!(peer_a_ack.header.ctl.ack());
        assert_eq!(peer_a_ack.header.seq, 101);
        assert_eq!(peer_a_ack.header.ack, 401);
    }

    // TODO(hardint): Add tests for the exchanges in figures 9 through 11 about
    // half-open connections

    fn established_pair() -> (Tcb, Tcb) {
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        let peer_a_syn = peer_a.segments().remove(0);
        let mut peer_b = handle_listen(
            peer_a_syn,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();
        let peer_b_syn_ack = peer_b.segments().remove(0);
        peer_a.segment_arrives(peer_b_syn_ack);
        let peer_a_ack = peer_a.segments().remove(0);
        peer_b.segment_arrives(peer_a_ack);
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

        let peer_a_fin = peer_a.segments().remove(0);
        assert!(peer_a_fin.header.ctl.fin());
        assert!(peer_a_fin.header.ctl.ack());
        assert_eq!(peer_a_fin.header.seq, 100);
        assert_eq!(peer_a_fin.header.ack, 300);

        peer_b.segment_arrives(peer_a_fin);
        assert_eq!(peer_b.state, State::CloseWait);

        // 3
        let peer_b_ack = peer_b.segments().remove(0);
        assert!(peer_b_ack.header.ctl.ack());
        assert_eq!(peer_b_ack.header.seq, 300);
        assert_eq!(peer_b_ack.header.ack, 101);

        peer_a.segment_arrives(peer_b_ack);
        assert_eq!(peer_a.state, State::FinWait2);

        // 4
        peer_b.close();
        assert_eq!(peer_b.state, State::LastAck);

        let peer_b_fin = peer_b.segments().remove(0);
        assert!(peer_b_fin.header.ctl.fin());
        assert!(peer_b_fin.header.ctl.ack());
        assert_eq!(peer_b_fin.header.seq, 300);
        assert_eq!(peer_b_fin.header.ack, 101);

        peer_a.segment_arrives(peer_b_fin);
        assert_eq!(peer_a.state, State::TimeWait);

        // 5
        let peer_a_ack = peer_a.segments().remove(0);
        assert!(peer_a_ack.header.ctl.ack());
        assert_eq!(peer_a_ack.header.seq, 101);
        assert_eq!(peer_a_ack.header.ack, 301);

        let receive_result = peer_b.segment_arrives(peer_a_ack);
        assert_eq!(receive_result, SegmentArrivesResult::Close);

        let timeout = peer_a.advance_time(MSL.mul_f32(2.1));
        assert_eq!(timeout, AdvanceTimeResult::CloseConnection);
    }

    #[ignore]
    #[test]
    fn simultaneous_close_sequence() {
        // This test implements the following exchange from 3.6, Figure 13:
        //
        //     TCP Peer A                                           TCP Peer B
        //
        // 1.  ESTABLISHED                                          ESTABLISHED
        //
        // 2.  (Close)                                              (Close)
        //     FIN-WAIT-1  --> <SEQ=100><ACK=300><CTL=FIN,ACK>  ... FIN-WAIT-1
        //                 <-- <SEQ=300><ACK=100><CTL=FIN,ACK>  <--
        //                 ... <SEQ=100><ACK=300><CTL=FIN,ACK>  -->
        //
        // 3.  CLOSING     --> <SEQ=101><ACK=301><CTL=ACK>      ... CLOSING
        //                 <-- <SEQ=301><ACK=101><CTL=ACK>      <--
        //                 ... <SEQ=101><ACK=301><CTL=ACK>      -->
        //
        // 4.  TIME-WAIT                                            TIME-WAIT
        //     (2 MSL)                                              (2 MSL)
        //     CLOSED                                               CLOSED

        // 1
        let (mut peer_a, mut peer_b) = established_pair();

        // 2
        peer_a.close();
        assert_eq!(peer_a.state, State::FinWait1);
        let fin_ack_a = peer_a.segments().remove(0);
        assert_eq!(fin_ack_a.header.seq, 100);
        assert_eq!(fin_ack_a.header.ack, 300);
        assert!(fin_ack_a.header.ctl.fin());
        assert!(fin_ack_a.header.ctl.ack());

        peer_b.close();
        assert_eq!(peer_b.state, State::FinWait1);
        let fin_ack_b = peer_a.segments().remove(0);
        assert_eq!(fin_ack_b.header.seq, 300);
        assert_eq!(fin_ack_b.header.ack, 100);
        assert!(fin_ack_b.header.ctl.fin());
        assert!(fin_ack_b.header.ctl.ack());

        // 3
        peer_a.segment_arrives(fin_ack_b);
        assert_eq!(peer_a.state, State::Closing);
        let ack_a = peer_a.segments().remove(0);
        assert_eq!(ack_a.header.seq, 101);
        assert_eq!(ack_a.header.ack, 301);
        assert!(ack_a.header.ctl.ack());

        peer_b.segment_arrives(fin_ack_a);
        assert_eq!(peer_b.state, State::Closing);
        let ack_b = peer_b.segments().remove(0);
        assert_eq!(ack_b.header.seq, 101);
        assert_eq!(ack_b.header.ack, 301);
        assert!(ack_b.header.ctl.ack());

        // 4
        peer_a.segment_arrives(ack_b);
        assert_eq!(peer_a.state, State::TimeWait);
        assert_eq!(
            peer_a.advance_time(MSL.mul_f32(2.1)),
            AdvanceTimeResult::CloseConnection
        );

        peer_b.segment_arrives(ack_a);
        assert_eq!(peer_b.state, State::TimeWait);
        assert_eq!(
            peer_b.advance_time(MSL.mul_f32(2.1)),
            AdvanceTimeResult::CloseConnection
        );
    }

    #[test]
    fn message_send() {
        let expected = b"Hello, world!";
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected));
        for outgoing in peer_a.segments() {
            peer_b.segment_arrives(outgoing);
        }
        let received = peer_b.receive();
        assert_eq!(expected, received.as_slice());
    }

    #[test]
    fn message_segmentation() {
        let expected: Vec<_> = std::iter::repeat(0)
            .enumerate()
            .map(|(i, _)| i as u8)
            .take(4000)
            .collect();
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected.clone()));
        let mut count = 0;
        for outgoing in peer_a.segments() {
            count += 1;
            peer_b.segment_arrives(outgoing);
        }
        let received = peer_b.receive();
        assert_eq!(count, 3);
        assert_eq!(expected, received);
    }

    #[test]
    fn large_message_transmission() {
        let expected: Vec<_> = std::iter::repeat(0)
            .enumerate()
            .map(|(i, _)| i as u8)
            .take(8000) // This is beyond our receive window now
            .collect();
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected.clone()));
        let mut received = vec![];
        while received.len() != expected.len() {
            for outgoing in peer_a.segments() {
                peer_b.segment_arrives(outgoing);
            }
            received.extend(peer_b.receive());
            for outgoing in peer_b.segments() {
                peer_a.segment_arrives(outgoing);
            }
            peer_a.advance_time(Duration::from_millis(1));
            peer_b.advance_time(Duration::from_millis(1));
        }
        assert_eq!(expected, received);
    }

    #[test]
    fn message_retransmission() {
        let expected: Vec<_> = (0..8000).map(|i| i as u8).collect();
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected.clone()));
        let mut received = vec![];
        while received.len() < expected.len() {
            for outgoing in peer_a.segments() {
                if rand::random::<f32>() < 0.5 {
                    peer_b.segment_arrives(outgoing);
                }
            }
            received.extend(peer_b.receive());
            for outgoing in peer_b.segments() {
                if rand::random::<f32>() < 0.5 {
                    peer_a.segment_arrives(outgoing);
                }
            }
            peer_a.advance_time(Duration::from_millis(1));
            peer_b.advance_time(Duration::from_millis(1));
        }
        assert_eq!(expected, received);
    }

    #[test]
    fn out_of_order_delivery() {
        let expected: Vec<_> = std::iter::repeat(0)
            .enumerate()
            .map(|(i, _)| i as u8)
            .take(4000)
            .collect();
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected.clone()));
        let segments = peer_a.segments();
        for outgoing in segments.into_iter().rev() {
            peer_b.segment_arrives(outgoing);
        }
        let received = peer_b.receive();
        assert_eq!(expected, received);
    }

    #[test]
    fn loss_during_initiation() {
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        peer_a.segments();
        peer_a.advance_time(Duration::from_secs(1));
        let peer_a_syn = peer_a.segments();
        assert_eq!(peer_a_syn.len(), 1);
        let peer_a_syn = peer_a_syn.into_iter().next().unwrap();

        let mut peer_b = handle_listen(
            peer_a_syn.clone(),
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();
        peer_b.segments();
        peer_b.advance_time(Duration::from_secs(1));
        let peer_b_syn_ack = peer_b.segments();
        assert_eq!(peer_b_syn_ack.len(), 1);
        let peer_b_syn_ack = peer_b_syn_ack.into_iter().next().unwrap();
        assert!(peer_b_syn_ack.header.ctl.syn());
        assert!(peer_b_syn_ack.header.ctl.ack());
        // Lost packet arrives
        assert_eq!(peer_b.segment_arrives(peer_a_syn), SegmentArrivesResult::Ok);

        assert_eq!(
            peer_a.segment_arrives(peer_b_syn_ack.clone()),
            SegmentArrivesResult::Ok
        );
        assert_eq!(peer_a.state, State::Established);
        // Lost, new ACK not generated
        let _peer_a_ack = peer_a.segments();

        assert_eq!(
            // Peer B probes again
            peer_a.segment_arrives(peer_b_syn_ack.clone()),
            SegmentArrivesResult::Ok
        );
        peer_a.advance_time(Duration::from_secs(1));
        let peer_a_ack = peer_a.segments();

        assert_eq!(peer_a_ack.len(), 1);
        let peer_a_ack = peer_a_ack.into_iter().next().unwrap();
        assert!(peer_a_ack.header.ctl.ack());
        assert!(!peer_a_ack.header.ctl.syn());
        // Lost packet arrives
        assert_eq!(
            peer_a.segment_arrives(peer_b_syn_ack),
            SegmentArrivesResult::Ok
        );

        assert_eq!(peer_b.segment_arrives(peer_a_ack), SegmentArrivesResult::Ok);
        assert_eq!(peer_b.state, State::Established);
    }

    #[test]
    fn send_before_established() {
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        peer_a.send(Message::new("Hello!"));
        let peer_a_syn = peer_a.segments().remove(0);
        let mut peer_b = handle_listen(
            peer_a_syn,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();
        peer_b.send(Message::new("Hi!"));
        for segment in peer_b.segments() {
            peer_a.segment_arrives(segment);
        }
        for segment in peer_a.segments() {
            peer_b.segment_arrives(segment);
        }
        assert_eq!(peer_a.state, State::Established);
        assert_eq!(peer_b.state, State::Established);
        assert_eq!(peer_a.receive(), b"Hi!");
        assert_eq!(peer_b.receive(), b"Hello!");
    }
}
