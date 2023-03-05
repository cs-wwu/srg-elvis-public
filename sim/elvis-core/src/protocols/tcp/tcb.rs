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
    sync::{RwLock, RwLockWriteGuard},
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
#[derive(Debug)]
pub struct Tcb {
    /// The pair of endpoints that identifies this connection
    id: ConnectionId,
    /// The maximum transmission unit of the network
    mtu: Mtu,
    /// How the connection was initiated locally
    initiation: Initiation,
    /// Variables for the connection state
    connection: RwLock<Connection>,
    /// Data and segments to be delivered to the remote TCP
    outgoing: Outgoing,
    /// Segments and segment text received from the remote TCP
    incoming: Incoming,
    /// Segments received from the remote TCP that have not been processed
    timeouts: Timeouts,
}

impl Tcb {
    /// Creates a new TCB
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
            connection: RwLock::new(Connection::new(state, snd, rcv)),
            outgoing: Default::default(),
            incoming: Default::default(),
            timeouts: Default::default(),
        }
    }

    /// Open a new TCP connection.
    ///
    /// Implements [section
    /// 3.10.1](https://www.rfc-editor.org/rfc/rfc9293.html#name-open-call) for
    /// the case of an active open. Handling for packets in a passive open
    /// LISTEN state is provided by [`handle_listen`].
    pub fn open(id: ConnectionId, iss: u32, mtu: Mtu) -> Self {
        let tcb = Self::new(
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
        tcb.enqueue(
            tcb.header_builder(iss)
                .syn()
                .wnd(ReceiveSequenceSpace::default().wnd),
        );
        tcb
    }

    /// Advance the current time and process any timeouts as needed.
    ///
    /// Timeout handling is described in [section
    /// 3.10.8](https://www.rfc-editor.org/rfc/rfc9293.html#name-timeouts).
    pub fn advance_time(&self, delta_time: Duration) -> AdvanceTimeResult {
        let mut retransmission = self.timeouts.retransmission.write().unwrap();
        if delta_time > *retransmission {
            *retransmission = RETRANSMISSION_TIMEOUT;
            for mut transmit in self.outgoing.retransmit.write().unwrap().iter_mut() {
                transmit.needs_transmit = true;
            }
        } else {
            *retransmission -= delta_time;
        }
        drop(retransmission);

        let mut time_wait_lock = self.timeouts.time_wait.write().unwrap();
        if let Some(time_wait) = *time_wait_lock {
            if delta_time > time_wait {
                return AdvanceTimeResult::CloseConnection;
            }
            *time_wait_lock = Some(time_wait - delta_time);
        }

        AdvanceTimeResult::Ignore
    }

    /// Send the provided message to the remote TCP.
    ///
    /// Implements [section
    /// 3.10.2](https://www.rfc-editor.org/rfc/rfc9293.html#name-send-call).
    pub fn send(&self, message: &Message) {
        // 3.10.2 (Not compliant, doing things differently. We don't have a
        // retransmission queue.)
        let state = self.connection.write().unwrap().state;
        match state {
            State::SynSent | State::SynReceived | State::Established => {
                self.outgoing.text.write().unwrap().concatenate(message);
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

    /// Consumes any segment text that has been received and buffered from the
    /// remote TCP.
    ///
    /// Implements [section
    /// 3.10.3](https://www.rfc-editor.org/rfc/rfc9293.html#name-receive-call).
    pub fn receive(&self) -> Message {
        // TODO(hardint): This currently requires copying bytes from the
        // received messages because we cannot yet concatenate two messages.
        // Revisit this when the message type has been updated to support
        // concatenation.
        let state = self.connection.write().unwrap().state;
        match state {
            State::SynSent
            | State::SynReceived
            | State::Established
            | State::FinWait1
            | State::FinWait2
            | State::CloseWait => {
                // TODO(hardint): Use receive buffer size instead of just taking
                // everything
                mem::take(&mut *self.incoming.text.write().unwrap())
            }
            State::Closing | State::LastAck | State::TimeWait => {
                // TODO(hardint): Return a connection closing error
                Default::default()
            }
        }
    }

    /// Initiates closing the TCP connection in a controlled way. No new data
    /// can be sent after this function has been called.
    ///
    /// Implements [section
    /// 3.10.4](https://www.rfc-editor.org/rfc/rfc9293.html#name-close-call).
    pub fn close(&self) -> CloseResult {
        let mut connection = self.connection.write().unwrap();
        match connection.state {
            State::SynReceived | State::Established => {
                self.enqueue(
                    self.header_builder(connection.snd.nxt)
                        .fin()
                        .ack(connection.rcv.nxt)
                        .wnd(connection.rcv.wnd),
                );
                connection.snd.nxt += 1;
                connection.state = State::FinWait1;
                CloseResult::Ok
            }

            State::CloseWait => {
                self.enqueue(
                    self.header_builder(connection.snd.nxt)
                        .fin()
                        .ack(connection.rcv.nxt)
                        .wnd(connection.rcv.wnd),
                );
                connection.snd.nxt += 1;
                connection.state = State::LastAck;
                CloseResult::Ok
            }

            _ => CloseResult::ConnectionClosing,
        }
    }

    /// Closes the connection immediately and without waiting for
    /// acknowledgement. The TCB should be deleted after this call once the
    /// final RST segment is delivered, if present.
    ///
    /// Implements [section
    /// 3.10.5](https://www.rfc-editor.org/rfc/rfc9293.html#section-3.10.5).
    pub fn abort(&self) {
        // 3.10.5
        let connection = self.connection.read().unwrap();
        match connection.state {
            State::SynReceived
            | State::Established
            | State::FinWait1
            | State::FinWait2
            | State::CloseWait => {
                self.outgoing.reset();
                self.enqueue(
                    self.header_builder(connection.snd.nxt)
                        .rst()
                        .wnd(connection.rcv.wnd),
                );
            }

            _ => {}
        }
    }

    /// The status of the TCP connection.
    ///
    /// Implements [section
    /// 3.10.6](https://www.rfc-editor.org/rfc/rfc9293.html#name-status-call).
    pub fn status(&self) -> State {
        // 3.10.6
        self.connection.read().unwrap().state
    }

    /// Gets the list of segments that are ready to be delivered to the remote
    /// TCP. Queued outgoing text is segmentized as needed and segments on the
    /// retransmission queue will not be resent until the next retransmission
    /// timeout.
    pub fn segments(&self) -> Vec<Segment> {
        let mut out: Vec<_> = mem::take(&mut *self.outgoing.oneshot.write().unwrap())
            .into_iter()
            .map(|header| Segment::new(header, [].into()))
            .collect();

        // TODO(hardint): Would love to make this locking more fine-grained
        let state = self.connection.read().unwrap().state;
        match state {
            State::SynSent | State::SynReceived | State::Established | State::CloseWait => {
                // TODO(hardint): This could be incorrect for when optional
                // headers are used. It also is not as efficient as possible.
                const SPACE_FOR_HEADERS: u32 = 50;
                let max_segment_length = (self.mtu - SPACE_FOR_HEADERS) as usize;
                let mut queued_bytes = self.outgoing.queued_bytes();
                loop {
                    let connection = self.connection.read().unwrap();
                    let max_bytes = connection.snd.wnd as usize - queued_bytes;
                    let mut text = self.outgoing.text.write().unwrap();
                    let bytes = max_segment_length.min(max_bytes).min(text.len());
                    if bytes == 0 {
                        break;
                    }
                    let mut new_text = text.clone();
                    text.slice(bytes..);
                    drop(text);
                    new_text.slice(..bytes);
                    queued_bytes += new_text.len();
                    let header = self
                        .header_builder(connection.snd.nxt)
                        .ack(connection.rcv.nxt)
                        .wnd(connection.rcv.wnd)
                        .build(
                            self.id.local.address,
                            self.id.remote.address,
                            new_text.iter(),
                        )
                        .expect("Unexpectedly large MTU and message");
                    drop(connection);
                    let mut connection = self.connection.write().unwrap();
                    connection.snd.nxt = connection.snd.nxt.wrapping_add(new_text.len() as u32);
                    drop(connection);
                    self.outgoing
                        .retransmit
                        .write()
                        .unwrap()
                        .push_back(Transmit::new(Segment::new(header, new_text)));
                }
            }

            _ => {}
        }

        for transmit in self.outgoing.retransmit.write().unwrap().iter_mut() {
            if transmit.needs_transmit {
                out.push(transmit.segment.clone());
            }
            transmit.needs_transmit = false;
        }

        if !out.is_empty() {
            *self.timeouts.retransmission.write().unwrap() = RETRANSMISSION_TIMEOUT;
        }

        out
    }

    /// Queues the given segment for processing and processes all segments that
    /// are ready to be processed.
    ///
    /// Along with [`Tcb::process_segment`], this implements [section
    /// 3.10.7](https://www.rfc-editor.org/rfc/rfc9293.html#name-segment-arrives)
    /// for all states other that CLOSED and LISTEN.
    pub fn segment_arrives(&self, segment: Segment) -> SegmentArrivesResult {
        let mut segments = self.incoming.segments.write().unwrap();
        segments.push(segment);
        while let Some(segment) = segments.peek() {
            let connection = self.connection.read().unwrap();
            if connection.state != State::SynSent && mod_gt(segment.header.seq, connection.rcv.nxt)
            {
                // If this segment is past the next byte we want to receive, it
                // arrived out of order and we haven't received the earlier
                // bytes we need to proceed.
                break;
            }
            drop(connection);
            let segment = segments.pop().unwrap();
            let receive_result = self.process_segment(segment);
            if receive_result != ProcessSegmentResult::Success {
                println!("{:?}", receive_result);
            }
            if receive_result.should_delete_tcb() {
                return SegmentArrivesResult::Close;
            }
        }
        SegmentArrivesResult::Ok
        // TODO(hardint): Aggregate ACK segments
    }

    /// Processes a segment.
    ///
    /// See 3.10.7.3 for handling in SYN-SENT state. See 3.10.7.4 for handling of the rest of the states.
    fn process_segment(&self, segment: Segment) -> ProcessSegmentResult {
        let (seg, mut text) = segment.into_inner();
        let text_len = text.len() as u32;
        let mut connection = self.connection.write().unwrap();

        // Check that the sequence number is valid
        match connection.state {
            // Sequence number checks don't apply for LISTEN, SYN-SENT, or CLOSING
            State::SynSent | State::Closing => {}
            _ => {
                if !self.is_seq_ok(&connection, text_len, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
                    self.enqueue(
                        self.header_builder(connection.snd.nxt)
                            .ack(connection.rcv.nxt)
                            .wnd(connection.rcv.wnd),
                    );
                    return ProcessSegmentResult::DiscardSegment;
                }
            }
        }

        if seg.ctl.ack() {
            match connection.state {
                State::SynSent => {
                    if mod_bounded(connection.snd.nxt, Lt, seg.ack, Leq, connection.snd.iss) {
                        if seg.ctl.rst() {
                            return ProcessSegmentResult::DiscardSegment;
                        } else {
                            self.enqueue(
                                self.header_builder(seg.ack).rst().wnd(connection.rcv.wnd),
                            );
                            return ProcessSegmentResult::InvalidAck;
                        }
                    }

                    if mod_bounded(connection.snd.una, Lt, seg.ack, Leq, connection.snd.nxt) {
                        // Valid acknowledgment
                        if seg.ctl.syn() {
                            // The spec doesn't specifically describe what to do
                            // for on okay ACK in SYN-SENT, but this seems to
                            // work
                            connection.snd.una = seg.ack;
                            self.remove_acked_from_retransmission(connection.snd.una);
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
                        // Same ACK twice causes this reset to trigger. See the
                        // comment above.
                        self.enqueue(self.header_builder(seg.ack).rst().wnd(connection.rcv.wnd));
                        return ProcessSegmentResult::InvalidAck;
                    }
                }

                State::SynReceived => {
                    if mod_bounded(connection.snd.una, Lt, seg.ack, Leq, connection.snd.nxt) {
                        connection.state = State::Established;
                        connection.snd.wnd = seg.wnd;
                        connection.snd.wl1 = seg.seq;
                        connection.snd.wl2 = seg.ack;
                        match self.ack_established_processing(&mut connection, &seg) {
                            ProcessSegmentResult::Success => {}
                            other => return other,
                        }
                    } else {
                        self.enqueue(self.header_builder(seg.ack).rst().wnd(connection.rcv.wnd));
                    }
                }

                State::Established | State::FinWait2 | State::CloseWait => {
                    match self.ack_established_processing(&mut connection, &seg) {
                        ProcessSegmentResult::Success => {}
                        other => return other,
                    }
                }

                State::FinWait1 => {
                    let result = self.ack_established_processing(&mut connection, &seg);
                    if self.is_fin_acked(&mut connection) {
                        connection.state = State::FinWait2;
                    }
                    if result != ProcessSegmentResult::Success {
                        return result;
                    }
                }

                State::Closing => {
                    let result = self.ack_established_processing(&mut connection, &seg);
                    if self.is_fin_acked(&mut connection) {
                        connection.state = State::TimeWait;
                        *self.timeouts.time_wait.write().unwrap() = Some(MSL * 2);
                    }
                    if result != ProcessSegmentResult::Success {
                        return result;
                    }
                }

                State::LastAck => {
                    connection.snd.una = seg.ack;
                    if self.is_fin_acked(&mut connection) {
                        return ProcessSegmentResult::FinalizeClose;
                    }
                }

                State::TimeWait => {
                    // The only thing that can arrive is a retransmission of the
                    // remote FIN. Acknowledge it and restart the 2 MSL timeout.
                    self.enqueue(
                        self.header_builder(connection.snd.nxt)
                            .ack(seg.seq + 1)
                            .wnd(connection.rcv.wnd),
                    );
                    *self.timeouts.time_wait.write().unwrap() = Some(MSL * 2);
                }
            }
        }

        if seg.ctl.rst() {
            match connection.state {
                State::SynSent => {
                    if seg.seq == connection.rcv.nxt {
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
                    return ProcessSegmentResult::ConnectionReset;
                }

                State::Closing | State::LastAck | State::TimeWait => {
                    return ProcessSegmentResult::FinalizeClose;
                }
            }
        }

        if seg.ctl.syn() {
            match connection.state {
                State::SynSent => {
                    connection.rcv.irs = seg.seq;
                    connection.rcv.nxt = seg.seq + 1;
                    connection.snd.wnd = seg.wnd;
                    connection.snd.wl1 = seg.seq;
                    connection.snd.wl2 = seg.ack;
                    if mod_gt(connection.snd.una, connection.snd.iss) {
                        connection.state = State::Established;
                        self.enqueue(
                            self.header_builder(connection.snd.nxt)
                                .ack(connection.rcv.nxt)
                                .wnd(connection.rcv.wnd),
                        );
                    } else {
                        connection.state = State::SynReceived;
                        self.enqueue(
                            self.header_builder(connection.snd.iss)
                                .syn()
                                .ack(connection.rcv.nxt)
                                .wnd(connection.rcv.wnd),
                        );
                        return ProcessSegmentResult::Success;
                    }
                }

                _ => {
                    // We are ignoring some of the spec's guidance around
                    // closing the connection if we get a SYN in an established
                    // state. It seems to create a lot of failed connections due
                    // to delayed SYN packets. We do a subset of what the spec
                    // suggests and just send a challenge ACK, which is
                    // important for the case where a peer generates an ACK in
                    // response to a SYN ACK and the ACK gets lost in
                    // transmission. The challenge ACK regenerates the lost ACK
                    // segment.
                    self.enqueue(
                        self.header_builder(connection.snd.nxt)
                            .ack(connection.rcv.nxt)
                            .wnd(connection.rcv.wnd),
                    );
                    return ProcessSegmentResult::DiscardSegment;
                }
            }
        }

        // Queue the segment text for processing
        if !text.is_empty() {
            match connection.state {
                State::Established
                | State::SynSent
                | State::SynReceived
                | State::FinWait1
                | State::FinWait2 => {
                    // If we got here, we already know that SEQ > RCV.NXT.
                    // Text should also be in the window, but let's check:
                    assert!(
                        self.is_in_rcv_window(&connection, seg.seq)
                            || self.is_in_rcv_window(&connection, seg.seq + text_len)
                    );
                    let already_received = connection
                        .rcv
                        .nxt
                        .wrapping_sub(seg.seq)
                        // SYN occupies the first byte of data
                        .wrapping_add(seg.ctl.syn() as u32);
                    let unreceived = text_len - already_received;
                    let mut incoming_text = self.incoming.text.write().unwrap();
                    let space_available = connection.rcv.wnd as u32 - incoming_text.len() as u32;
                    let accept = unreceived.min(space_available);
                    connection.rcv.nxt += accept;
                    text.slice(already_received as usize..(already_received + accept) as usize);
                    incoming_text.concatenate(&text);
                    drop(incoming_text);
                    // TODO(hardint): Aggregate and piggyback ACK segments
                    self.enqueue(
                        self.header_builder(connection.snd.nxt)
                            .ack(connection.rcv.nxt)
                            .wnd(connection.rcv.wnd),
                    );
                }

                _ => {}
            }
        }

        if seg.ctl.fin() {
            if connection.state != State::SynSent {
                let last_text_byte = seg.seq + text_len;
                if connection.rcv.nxt == last_text_byte || connection.rcv.nxt == last_text_byte + 1
                {
                    // We acknowledged all the non-control bytes in the segment or we
                    // have already acknowledged the FIN. Advance over the FIN and
                    // acknowledge it.
                    connection.rcv.nxt = last_text_byte + 1;
                    self.enqueue(
                        self.header_builder(connection.snd.nxt)
                            .ack(connection.rcv.nxt)
                            .wnd(connection.rcv.wnd),
                    );
                }
            }

            match connection.state {
                State::SynReceived | State::Established => {
                    connection.state = State::CloseWait;
                }

                State::FinWait1 => {
                    if self.is_fin_acked(&connection) {
                        connection.state = State::TimeWait;
                        *self.timeouts.time_wait.write().unwrap() = Some(2 * MSL);
                    } else {
                        connection.state = State::Closing;
                    }
                }

                State::FinWait2 => {
                    connection.state = State::TimeWait;
                    *self.timeouts.time_wait.write().unwrap() = Some(2 * MSL);
                    *self.timeouts.retransmission.write().unwrap() = RETRANSMISSION_TIMEOUT;
                }

                State::TimeWait => {
                    *self.timeouts.time_wait.write().unwrap() = Some(2 * MSL);
                }

                _ => {}
            }
        }

        ProcessSegmentResult::Success
    }

    /// Remove any acknowledged segments from the retransmission queue.
    fn remove_acked_from_retransmission(&self, snd_una: u32) {
        let mut retransmit = self.outgoing.retransmit.write().unwrap();
        let mut i = 0;
        while let Some(transmit) = retransmit.get(i) {
            let seq = transmit.segment.header.seq;
            let seg_len = transmit.segment.seg_len() as u32;
            if mod_lt(snd_una, seq + seg_len) {
                i += 1;
            } else {
                retransmit.remove(i);
            }
        }
    }

    /// Whether our FIN has been acknowledged by the remote TCP
    fn is_fin_acked(&self, connection: &RwLockWriteGuard<Connection>) -> bool {
        connection.snd.nxt == connection.snd.una
    }

    /// Processing for ACK segments in the ESTABLISHED state, as described in
    /// 3.10.7.4.
    ///
    /// This is factored out from the mainline segment processing code because
    /// it is used by several different states.
    fn ack_established_processing(
        &self,
        connection: &mut RwLockWriteGuard<Connection>,
        seg: &TcpHeader,
    ) -> ProcessSegmentResult {
        if mod_leq(seg.ack, connection.snd.una) {
            // Ignore duplicate ACK
            return ProcessSegmentResult::Success;
        } else if mod_gt(seg.ack, connection.snd.nxt) {
            // ACKs something not yet sent
            self.enqueue(
                self.header_builder(connection.snd.nxt)
                    .ack(connection.rcv.nxt)
                    .wnd(connection.rcv.wnd),
            );
            return ProcessSegmentResult::InvalidAck;
        } else {
            // Valid ACK
            connection.snd.una = seg.ack;
            self.remove_acked_from_retransmission(connection.snd.una);
            if mod_lt(connection.snd.wl1, seg.seq)
                || (connection.snd.wl1 == seg.seq && mod_leq(connection.snd.wl2, seg.ack))
            {
                // Update the send window
                connection.snd.wnd = seg.wnd;
                connection.snd.wl1 = seg.seq;
                connection.snd.wl2 = seg.ack;
            }
        }
        ProcessSegmentResult::Success
    }

    /// Get a TCP header builder for the connection
    fn header_builder(&self, seq: u32) -> TcpHeaderBuilder {
        TcpHeaderBuilder::new(self.id.local.port, self.id.remote.port, seq)
    }

    /// Queue a segment without segment text for transmission. SYN and FIN
    /// segments may be retransmitted.
    fn enqueue(&self, header_builder: TcpHeaderBuilder) {
        let header = header_builder
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
                .write()
                .unwrap()
                .push_back(Transmit::new(Segment::new(header, [].into())));
        } else {
            self.outgoing.oneshot.write().unwrap().push(header);
        }
    }

    /// Checks whether a sequence number is valid as described in [section
    /// 3.4](https://www.rfc-editor.org/rfc/rfc9293.html#name-sequence-numbers).
    ///
    /// Note however, the original design for sequence number validation fails
    /// under certain situations, such as simultaneous open. Appendix A.2 links
    /// to a
    /// [revision](https://datatracker.ietf.org/doc/html/draft-gont-tcpm-tcp-seq-validation-04)
    /// to sequence number validation that we employ. See page 10 for the
    /// updated procedure.
    fn is_seq_ok(
        &self,
        connection: &RwLockWriteGuard<Connection>,
        data_len: u32,
        seq: u32,
        syn: bool,
        fin: bool,
    ) -> bool {
        let seg_len = data_len + fin as u32 + syn as u32;
        // Test segment acceptability. See Table 6.
        if seg_len == 0 {
            if connection.rcv.wnd == 0 {
                mod_bounded(connection.rcv.nxt - 1, Leq, seq, Leq, connection.rcv.nxt)
            } else {
                self.is_in_rcv_window(connection, seq)
            }
        } else if connection.rcv.wnd == 0 {
            // When the receive window is zero, only ACKs are acceptible.
            false
        } else {
            self.is_in_rcv_window(connection, seq)
                || self.is_in_rcv_window(connection, seq + seg_len - 1)
        }
    }

    /// Whether a sequence number is in the receive window, as described in the
    /// revision to sequence number validation linked above.
    fn is_in_rcv_window(&self, connection: &RwLockWriteGuard<Connection>, n: u32) -> bool {
        mod_bounded(
            connection.rcv.nxt - 1,
            Leq,
            n,
            Lt,
            connection.rcv.nxt + connection.rcv.wnd as u32,
        )
    }
}

/// Handles the arrival of a segment in the CLOSED state.
///
/// Implements [section
/// 3.10.7.1](https://www.rfc-editor.org/rfc/rfc9293.html#name-closed-state).
pub fn segment_arrives_closed(
    seg: TcpHeader,
    text_len: u32,
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
            .ack(seg.seq + text_len)
    }
    .build(local, remote, [].into_iter())
    .ok()
}

/// Handles the arrival of a segment in the LISTEN state.
///
/// Implements [section
/// 3.10.7.2](https://www.rfc-editor.org/rfc/rfc9293.html#name-listen-state).
pub fn segment_arrives_listen(
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
        let rcv_nxt = seg.seq + 1;
        let tcb = Tcb::new(
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
                nxt: rcv_nxt,
                ..Default::default()
            },
        );
        tcb.enqueue(
            tcb.header_builder(iss)
                .syn()
                .ack(rcv_nxt)
                .wnd(ReceiveSequenceSpace::default().wnd),
        );

        // Processing of SYN and ACK should not be repeated.
        seg.ctl.set_syn(false);
        seg.ctl.set_ack(false);
        tcb.incoming
            .segments
            .write()
            .unwrap()
            .push(Segment::new(seg, message));

        Some(ListenResult::Tcb(tcb))
    } else {
        // Fourth:
        // Any other control or data-bearing segment should be discarded
        None
    }
}

/// Removes and aggregates the given number of bytes from a FIFO queue of
/// messages.
fn consume_text(queue: &mut VecDeque<Message>, bytes: usize) -> Vec<u8> {
    let mut out = vec![];
    while let Some(text) = queue.front_mut() {
        if text.len() <= bytes {
            out.extend(text.iter());
            queue.pop_front();
        } else {
            out.extend(text.iter().take(bytes));
            text.slice(bytes..);
            break;
        }
    }
    out
}

/// Timeouts used by TCP
#[derive(Debug)]
struct Timeouts {
    /// The retransmission timeout
    retransmission: RwLock<Duration>,
    /// The time wait timeout
    time_wait: RwLock<Option<Duration>>,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            retransmission: RwLock::new(RETRANSMISSION_TIMEOUT),
            time_wait: Default::default(),
        }
    }
}

/// Segments and segment text received from the remote TCP
#[derive(Debug, Default)]
struct Incoming {
    /// Segments due for processing. Due to the comparison implementations on
    /// [`Segment`], elements will be removed in sequence number order.
    segments: RwLock<BinaryHeap<Segment>>,
    /// Segment text that has been aggregated from processed segments and is
    /// ready to be delivered to the user.
    text: RwLock<Message>,
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
        matches!(
            self,
            ProcessSegmentResult::ReturnToListen
                | ProcessSegmentResult::ConnectionReset
                | ProcessSegmentResult::ConnectionRefused
                | ProcessSegmentResult::FinalizeClose
                | ProcessSegmentResult::BlindReset
        )
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
#[allow(clippy::large_enum_variant)]
#[must_use]
#[derive(Debug)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Connection {
    /// The state of the connection
    pub state: State,
    /// The send sequence space
    pub snd: SendSequenceSpace,
    /// The receive sequence space
    pub rcv: ReceiveSequenceSpace,
}

impl Connection {
    pub fn new(state: State, snd: SendSequenceSpace, rcv: ReceiveSequenceSpace) -> Self {
        Self { state, snd, rcv }
    }
}
