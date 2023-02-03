use self::incoming::Incoming;

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

mod incoming;

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
                let bytes = self.received_text.iter().map(|message| message.len()).sum();
                consume_text(&mut self.received_text, bytes)
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
            if self.state != State::SynSent && mod_ge(segment.header.seq, self.rcv.nxt) {
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
                        self.time_wait_timeout = Some(MSL * 2);
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
                    self.time_wait_timeout = Some(MSL * 2);
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
                    self.received_text.push_back(text);
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
                        self.time_wait_timeout = Some(2 * MSL);
                    } else {
                        self.state = State::Closing;
                    }
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

    fn is_fin_acked(&self) -> bool {
        self.snd.nxt == self.snd.una
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

#[derive(Debug, Clone)]
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
