//! This module implements the Transmission Control Protocol as described in
//! [RFC 9293](https://www.rfc-editor.org/rfc/rfc9293.html), the update to the
//! original RFC 793 specification. [`Tcb`] provides the API described in
//! section 3.10 and is implemented separately from the TCP protocol and session
//! types so that it can be more easily tested outside of the full simulation
//! environment.

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
    collections::{BinaryHeap, VecDeque},
    mem,
    time::Duration,
};

#[cfg(test)]
mod tests;

mod modular_cmp;
use modular_cmp::*;

mod segment;
pub use segment::Segment;

mod outgoing;
use outgoing::{Outgoing, Transmit};

mod state;
pub use state::State;

mod receive_sequence_space;
use receive_sequence_space::ReceiveSequenceSpace;

mod send_sequence_space;
use send_sequence_space::SendSequenceSpace;

// TODO(hardint): Choose a more realistic value
/// The maximum segment lifetime on the Internet
const MSL: Duration = Duration::from_secs(1);

// TODO(hardint): Choose a better value
/// The time that may pass before packets are retransmitted
const RETRANSMISSION_TIMEOUT: Duration = Duration::from_millis(100);

/// The Transmission Control Block holds the state for a TCP connection and
/// provides the API described in 3.10.
#[derive(Debug, Clone)]
pub struct Tcb {
    /// The pair of endpoints that identifies this connection
    id: ConnectionId,
    /// The maximum transmission unit of the network
    mtu: Mtu,
    /// How the connection was initiated locally
    initiation: Initiation,
    /// The state of the connection
    state: State,
    /// The send sequence space
    snd: SendSequenceSpace,
    /// The receive sequence space
    rcv: ReceiveSequenceSpace,
    /// Data and segments to be delivered to the remote TCP
    outgoing: Outgoing,
    /// Segments and segment text received from the remote TCP
    incoming: Incoming,
    /// Segments received from the remote TCP that have not been processed
    timeouts: Timeouts,
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
            timeouts: Default::default(),
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
        if delta_time > self.timeouts.retransmission {
            self.timeouts.retransmission = RETRANSMISSION_TIMEOUT;
            for mut transmit in self.outgoing.retransmit.iter_mut() {
                transmit.needs_transmit = true;
            }
        } else {
            self.timeouts.retransmission -= delta_time;
        }

        if let Some(time_wait) = self.timeouts.time_wait {
            if delta_time > time_wait {
                return AdvanceTimeResult::CloseConnection;
            }
            self.timeouts.time_wait = Some(time_wait - delta_time);
        }

        AdvanceTimeResult::Ignore
    }

    pub fn send(&mut self, message: Message) {
        // 3.10.2 (Not compliant, doing things differently. We don't have a
        // retransmission queue.)
        match self.state {
            State::SynSent | State::SynReceived | State::Established => {
                self.outgoing.text.push_back(message);
            }

            State::FinWait1
            | State::FinWait2
            | State::CloseWait
            | State::Closing
            | State::LastAck
            | State::TimeWait => {
                // TODO(hardint): Return an error that the connection is closing
            }
        }
    }

    pub fn receive(&mut self) -> Vec<u8> {
        // 3.10.3
        match self.state {
            State::SynSent
            | State::SynReceived
            | State::Established
            | State::FinWait1
            | State::FinWait2
            | State::CloseWait => {
                // TODO(hardint): Use receive buffer size instead of just taking
                // everything
                let bytes = self.incoming.text.iter().map(|message| message.len()).sum();
                consume_text(&mut self.incoming.text, bytes)
            }
            State::Closing | State::LastAck | State::TimeWait => {
                // TODO(hardint): Return a connection closing error
                vec![]
            }
        }
    }

    pub fn close(&mut self) -> CloseResult {
        // 3.10.4
        match self.state {
            State::SynReceived | State::Established => {
                // TODO(hardint): Should only do this if there is no pending
                // data to send, but I don't have a good mechanism for queuing
                // things like this for later processing once we reach
                // ESTABLISHED as the spec describes, so this is how it is for
                // now.
                self.enqueue(self.header_builder(self.snd.nxt).fin().ack(self.rcv.nxt));
                self.snd.nxt += 1;
                self.state = State::FinWait1;
                CloseResult::Ok
            }

            State::CloseWait => {
                self.enqueue(self.header_builder(self.snd.nxt).fin().ack(self.rcv.nxt));
                self.snd.nxt += 1;
                self.state = State::LastAck;
                CloseResult::Ok
            }

            State::SynSent
            | State::FinWait1
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
        match self.state {
            State::SynReceived
            | State::Established
            | State::FinWait1
            | State::FinWait2
            | State::CloseWait => {
                self.outgoing = Default::default();
                self.enqueue(self.header_builder(self.snd.nxt).rst());
            }

            State::SynSent | State::Closing | State::LastAck | State::TimeWait => {}
        }
    }

    pub fn status(&self) -> State {
        // 3.10.6
        self.state
    }

    pub fn segments(&mut self) -> Vec<Segment> {
        let mut out: Vec<_> = mem::take(&mut self.outgoing.oneshot)
            .into_iter()
            .map(|header| Segment::new(header, [].into()))
            .collect();

        match self.state {
            State::SynSent | State::SynReceived | State::Established | State::CloseWait => {
                // TODO(hardint): This could be incorrect for when optional
                // headers are used. It also is not as efficient as possible.
                const SPACE_FOR_HEADERS: u32 = 50;
                let max_segment_length = (self.mtu - SPACE_FOR_HEADERS) as usize;
                let mut queued_bytes = self.outgoing.queued_bytes();
                loop {
                    let max_bytes = self.snd.wnd as usize - queued_bytes;
                    let text =
                        consume_text(&mut self.outgoing.text, max_segment_length.min(max_bytes));
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
            }

            _ => {}
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
            if mod_lt(self.snd.una, seq + seg_len) {
                i += 1;
            } else {
                self.outgoing.retransmit.remove(i);
            }
        }
    }

    pub fn segment_arrives(&mut self, segment: Segment) -> SegmentArrivesResult {
        self.incoming.segments.push(segment);
        while let Some(segment) = self.incoming.segments.peek() {
            if self.state != State::SynSent && mod_gt(segment.header.seq, self.rcv.nxt) {
                // If this segment is past the next byte we want to receive, it
                // arrived out of order and we haven't received the earlier
                // bytes we need to proceed.
                break;
            }
            let segment = self.incoming.segments.pop().unwrap();
            let receive_result = self.process_segment(segment);
            if receive_result.should_delete_tcb() {
                return SegmentArrivesResult::Close;
            }
        }
        SegmentArrivesResult::Ok
        // TODO(hardint): Aggregate ACK segments
    }

    fn process_segment(&mut self, segment: Segment) -> ProcessSegmentResult {
        let (seg, mut text) = segment.into_inner();
        let text_len = text.len() as u32;

        match self.state {
            // Sequence number checks don't apply for LISTEN, SYN-SENT, or CLOSING
            State::SynSent | State::Closing => {}
            _ => {
                if !self.is_seq_ok(text_len, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                    return ProcessSegmentResult::DiscardSegment;
                }
            }
        }

        if seg.ctl.ack() {
            match self.state {
                State::SynSent => {
                    if mod_bounded(self.snd.nxt, Lt, seg.ack, Leq, self.snd.iss) {
                        if seg.ctl.rst() {
                            // Discard the segment
                            return ProcessSegmentResult::DiscardSegment;
                        } else {
                            self.enqueue(self.header_builder(seg.ack).rst());
                            return ProcessSegmentResult::InvalidAck;
                        }
                    }

                    if mod_bounded(self.snd.una, Lt, seg.ack, Leq, self.snd.nxt) {
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
                    if mod_bounded(self.snd.una, Lt, seg.ack, Leq, self.snd.nxt) {
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
                State::Established | State::FinWait2 | State::CloseWait => {
                    match self.ack_established_processing(&seg) {
                        ProcessSegmentResult::Success => {}
                        other => return other,
                    }
                }

                State::FinWait1 => {
                    let result = self.ack_established_processing(&seg);
                    if self.is_fin_acked() {
                        self.state = State::FinWait2;
                    }
                    if result != ProcessSegmentResult::Success {
                        return result;
                    }
                }

                State::Closing => {
                    let result = self.ack_established_processing(&seg);
                    if self.is_fin_acked() {
                        self.state = State::TimeWait;
                        self.timeouts.time_wait = Some(MSL * 2);
                    }
                    if result != ProcessSegmentResult::Success {
                        return result;
                    }
                }

                State::LastAck => {
                    self.snd.una = seg.ack;
                    if self.is_fin_acked() {
                        return ProcessSegmentResult::FinalizeClose;
                    }
                }

                State::TimeWait => {
                    // The only thing that can arrive is a retransmission of the
                    // remote FIN. Acknowledge it and restart the MSL 2 timeout.
                    self.enqueue(self.header_builder(self.snd.nxt).ack(seg.seq + 1));
                    self.timeouts.time_wait = Some(MSL * 2);
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
                    if mod_gt(self.snd.una, self.snd.iss) {
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
                        self.is_in_rcv_window(seg.seq) || self.is_in_rcv_window(seg.seq + text_len)
                    );
                    let already_received = self
                        .rcv
                        .nxt
                        .wrapping_sub(seg.seq)
                        // SYN occupies the first byte of data
                        .wrapping_add(seg.ctl.syn() as u32);
                    let unreceived = text_len - already_received;
                    // TODO(hardint): Account for data already buffered
                    let accept = unreceived.min(self.rcv.wnd as u32);
                    self.rcv.nxt += accept;
                    text.slice(already_received as usize..(already_received + accept) as usize);
                    self.incoming.text.push_back(text);
                    // TODO(hardint): Aggregate and piggyback ACK segments
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                }

                State::CloseWait | State::Closing | State::LastAck | State::TimeWait => {
                    // Ignore the segment text
                }
            }
        }

        if seg.ctl.fin() {
            if self.state != State::SynSent {
                let last_text_byte = seg.seq + text_len;
                if self.rcv.nxt == last_text_byte || self.rcv.nxt == last_text_byte + 1 {
                    // We acknowledged all the non-control bytes in the segment or we
                    // have already acknowledged the FIN. Advance over the FIN and
                    // acknowledge it.
                    self.rcv.nxt = last_text_byte + 1;
                    self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
                }
            }

            match self.state {
                State::SynSent | State::CloseWait | State::Closing | State::LastAck => {}

                State::SynReceived | State::Established => {
                    self.state = State::CloseWait;
                }

                State::FinWait1 => {
                    if self.is_fin_acked() {
                        self.state = State::TimeWait;
                        self.timeouts.time_wait = Some(2 * MSL);
                    } else {
                        self.state = State::Closing;
                    }
                }

                State::FinWait2 => {
                    self.state = State::TimeWait;
                    // Start the time-wait timer, turn off the other timers.
                    self.timeouts.time_wait = Some(2 * MSL);
                    self.timeouts.retransmission = RETRANSMISSION_TIMEOUT;
                }

                State::TimeWait => {
                    // Restart the 2 MSL time-wait timeout.
                    self.timeouts.time_wait = Some(2 * MSL);
                }
            }
        }

        ProcessSegmentResult::Success
    }

    fn is_fin_acked(&self) -> bool {
        self.snd.nxt == self.snd.una
    }

    fn ack_established_processing(&mut self, seg: &TcpHeader) -> ProcessSegmentResult {
        if mod_leq(seg.ack, self.snd.una) {
            // Ignore duplicate ACK
            return ProcessSegmentResult::Success;
        } else if mod_gt(seg.ack, self.snd.nxt) {
            // ACKs something not yet sent
            self.enqueue(self.header_builder(self.snd.nxt).ack(self.rcv.nxt));
            return ProcessSegmentResult::InvalidAck;
        } else {
            // Valid ACK
            self.snd.una = seg.ack;
            self.remove_acked_from_retransmission();
            // Update the send window
            if mod_lt(self.snd.wl1, seg.seq)
                || (self.snd.wl1 == seg.seq && mod_leq(self.snd.wl2, seg.ack))
            {
                self.snd.wnd = seg.wnd;
                self.snd.wl1 = seg.seq;
                self.snd.wl2 = seg.ack;
            }
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
        mod_bounded(self.snd.una, Lt, ack, Leq, self.snd.nxt)
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
            Lt,
            self.rcv.nxt + self.rcv.wnd as u32,
        )
    }

    fn is_in_snd_window(&self, n: u32) -> bool {
        mod_bounded(self.snd.nxt, Leq, n, Lt, self.snd.nxt + self.snd.wnd as u32)
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
        tcb.incoming.segments.push(Segment::new(seg, message));

        Some(ListenResult::Tcb(tcb))
    } else {
        // Fourth:
        // Any other control or data-bearing segment should be discarded
        None
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

/// How the TCP connection was opened locally
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Initiation {
    /// The TCP created the connection after a passive open
    Listen,
    /// The TCB was created by an active open to a remote TCP
    Open,
}

/// The result of processing a TCP segment
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessSegmentResult {
    /// The segment processed successfully
    Success,
    /// The TCB threw away the bad segment, usually due to an invalid sequence
    /// number
    DiscardSegment,
    /// The segment carried an unacceptable ACK and was not fully processed
    InvalidAck,
    /// The TCP should return the connection to a LISTEN state
    ReturnToListen,
    /// The connection was reset
    ConnectionReset,
    /// The remote TCP refused the connection
    ConnectionRefused,
    /// The segment acknowledged the closing of the connection
    FinalizeClose,
    /// A potential blind reset attack was identified
    BlindReset,
}

impl ProcessSegmentResult {
    pub fn should_delete_tcb(self) -> bool {
        match self {
            ProcessSegmentResult::Success
            | ProcessSegmentResult::DiscardSegment
            | ProcessSegmentResult::InvalidAck => false,
            _ => true,
        }
    }
}

/// The result of a segment arriving on the TCB
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentArrivesResult {
    /// The segment was processed successfully
    Ok,
    /// The segment caused the TCB to close and the caller should delete the TCB
    Close,
}

/// The result of a call to send on the TCB
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    /// The send completed successfully
    Ok,
    /// The send did not complete because the connection is already closing
    ClosingConnection,
}

/// The result of a call to close on the TCB
#[must_use]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CloseResult {
    /// The close was processed successfully
    Ok,
    /// The connection is already closing
    ConnectionClosing,
    /// The TCB should be deleted by the caller
    CloseConnection,
}

/// The result of a segment arriving to the TCP in a LISTEN state
#[must_use]
#[derive(Debug, Clone)]
pub enum ListenResult {
    /// The connection attempt was processed successfully and a TCB was created
    /// for the connection
    Tcb(Tcb),
    /// The connection attempt failed and the TCP generated a response header
    /// instead of creating a TCB
    Response(TcpHeader),
}

impl ListenResult {
    /// Gets the response header, if available
    fn response(self) -> Option<TcpHeader> {
        match self {
            ListenResult::Response(response) => Some(response),
            ListenResult::Tcb(_) => None,
        }
    }

    /// Gets the TCB, if available
    fn tcb(self) -> Option<Tcb> {
        match self {
            ListenResult::Response(_) => None,
            ListenResult::Tcb(tcb) => Some(tcb),
        }
    }
}

/// The result of advancing the TCB time
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceTimeResult {
    /// The time was advanced and the caller needn't respond
    Ignore,
    /// The TCB closed as a result of advancing the time and the caller should
    /// delete the TCB
    CloseConnection,
}

/// Timeouts used by TCP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timeouts {
    /// The retransmission timeout
    retransmission: Duration,
    /// The time wait timeout
    time_wait: Option<Duration>,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            retransmission: RETRANSMISSION_TIMEOUT,
            time_wait: None,
        }
    }
}

/// Segments and segment text received from the remote TCP
#[derive(Debug, Clone, Default)]
struct Incoming {
    /// Segments due for processing. Due to the comparison implementations on
    /// [`Segment`], elements will be removed in sequence number order.
    segments: BinaryHeap<Segment>,
    /// Segment text that has been aggregated from processed segments and is
    /// ready to be delivered to the user.
    text: VecDeque<Message>,
}
