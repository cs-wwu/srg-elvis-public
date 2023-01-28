use super::{
    tcp_parsing::{BuildHeaderError, TcpHeader, TcpHeaderBuilder},
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
    mtu: Mtu,
    initiation: Initiation,
    state: State,
    snd: SendSequenceSpace,
    rcv: ReceiveSequenceSpace,
    retransmission_queue: VecDeque<Outgoing>,
    incoming: BinaryHeap<Incoming>,
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
            retransmission_queue: Default::default(),
            incoming: Default::default(),
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
        tcb.enqueue_outgoing(tcb.header_builder(iss).syn(), [].into())
            .unwrap();
        tcb
    }

    pub fn advance_time(&mut self, delta_time: Duration) -> AdvanceTimeResult {
        if delta_time > self.retransmission_timeout {
            self.retransmission_timeout = RETRANSMISSION_TIMEOUT;
            for outgoing in self.retransmission_queue.iter_mut() {
                outgoing.needs_retransmission = true;
            }
        } else {
            self.retransmission_timeout = self.retransmission_timeout - delta_time;
        }

        if let Some(time_wait) = self.time_wait_timeout {
            if delta_time > time_wait {
                return AdvanceTimeResult::CloseConnection;
            }
            self.time_wait_timeout = Some(time_wait - delta_time);
        }

        AdvanceTimeResult::Ignore
    }

    pub fn send(&mut self, mut message: Message) -> Result<SendResult, BuildHeaderError> {
        // 3.10.2

        // TODO(hardint): Should only queue up parts for transmission up to the
        // send window, then wait until space is available to queue up more.

        match self.state {
            State::SynSent | State::SynReceived | State::Established | State::CloseWait => {
                // TODO(hardint): This could be incorrect for when optional
                // headers are used. It also is not as efficient as possible.
                const SPACE_FOR_HEADERS: u32 = 50;

                let max_segment_length = (self.mtu - SPACE_FOR_HEADERS) as usize;
                while message.len() > max_segment_length {
                    let mut copy = message.clone();
                    copy.slice(..max_segment_length);
                    message.slice(max_segment_length..);
                    self.enqueue_outgoing(
                        self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                        copy,
                    )?;
                    self.snd.nxt = self.snd.nxt.wrapping_add(max_segment_length as u32);
                }
                let message_len = message.len();
                self.enqueue_outgoing(
                    self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                    message,
                )?;
                self.snd.nxt = self.snd.nxt.wrapping_add(message_len as u32);
                Ok(SendResult::Ok)
            }

            State::FinWait1
            | State::FinWait2
            | State::Closing
            | State::LastAck
            | State::TimeWait => Ok(SendResult::ClosingConnection),
        }
    }

    pub fn receive(&mut self) -> Vec<u8> {
        // 3.10.3
        let mut out = vec![];
        // This processes in order of sequence numbers by using a priority queue
        // with the Incoming type
        while let Some(segment) = self.incoming.peek() {
            let Incoming { seg, message } = segment;
            if !self.is_seq_ok(message.len() as u32, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
                self.incoming.pop();
                continue;
            }

            if mod_ge(seg.seq, self.rcv.nxt) {
                // If this segment is past the next byte we want to receive, we
                // haven't received the earlier bytes we need to proceed.
                break;
            }

            let bytes_already_received = self.rcv.nxt - seg.seq; // Works with modulus
            self.rcv.nxt =
                seg.seq + message.len() as u32 + seg.ctl.syn() as u32 + seg.ctl.fin() as u32;
            out.extend(message.iter().skip(bytes_already_received as usize));
            self.incoming.pop();
        }
        // TODO(hardint): Piggyback acknowledgement
        self.enqueue_outgoing(
            self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
            Message::new(vec![]),
        )
        .unwrap(); // Shouldn't fail for short messages
        out
    }

    fn remove_acked_from_retransmission(&mut self) {
        while let Some(segment) = self.retransmission_queue.front() {
            if !self.is_in_snd_window(segment.seg.seq) {
                self.retransmission_queue.pop_front();
            } else {
                break;
            }
        }
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
                self.enqueue_outgoing(self.header_builder(self.snd.nxt + 1), Message::new(vec![]))
                    .unwrap(); // For short segments this is fine
                self.state = State::FinWait1;
                CloseResult::Ok
            }

            State::CloseWait => {
                self.enqueue_outgoing(self.header_builder(self.snd.nxt + 1), Message::new(vec![]))
                    .unwrap(); // For short segments this is fine
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
        self.retransmission_queue = Default::default();
        if self.state == State::CloseWait {
            self.enqueue_outgoing(
                self.header_builder(self.snd.nxt).rst(),
                Message::new(vec![]),
            )
            .unwrap(); // Okay for short message
        }
    }

    pub fn status(&self) -> State {
        // 3.10.6
        self.state
    }

    pub fn outgoing(&mut self) -> impl Iterator<Item = Outgoing> + '_ {
        // This indicates which messages are ready to send. Prepare this in a
        // loop and move it into the iterator to avoid having to store which
        // messages should be transmitted directly on the Outgoing struct.
        let mut mask = 0u64;
        for (i, outgoing) in self.retransmission_queue.iter_mut().take(64).enumerate() {
            mask |= (outgoing.needs_retransmission as u64) << i;
            outgoing.needs_retransmission = false;
        }

        self.retransmission_queue
            .iter()
            .take(64)
            .enumerate()
            .filter_map(move |(i, outgoing)| ((mask >> i) & 1 == 1).then_some(outgoing))
            .cloned()
    }

    pub fn segment_arrives(
        &mut self,
        seg: TcpHeader,
        message: Message,
    ) -> Result<ReceiveResult, ReceiveError> {
        // TODO(hardint): Remove ACKed segments from the retransmission queue
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
                        // Remove acknowledged segments from retransmission queue
                        while let Some(segment) = self.retransmission_queue.front() {
                            // NOTE(hardint): Should this be leq?
                            if mod_le(segment.seg.seq + segment.message.len() as u32, self.snd.una)
                            {
                                self.retransmission_queue.pop_front();
                            } else {
                                break;
                            }
                        }
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
                }

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

                    if seg.ctl.ack() {
                        self.snd.una = seg.ack;
                    }

                    self.remove_acked_from_retransmission();

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

        // TODO(hardint): Should this check be happening earlier? Should it be
        // happening later when queued segments are processed?
        if !self.is_seq_ok(message.len() as u32, seg.seq, seg.ctl.syn(), seg.ctl.fin()) {
            self.enqueue_outgoing(
                self.header_builder(self.snd.nxt).ack(self.rcv.nxt),
                [].into(),
            )?;
            return Ok(ReceiveResult::DiscardSegment);
        }

        // Queue the segment text for processing
        match self.state {
            State::Established | State::FinWait1 | State::FinWait2 => {
                if !message.is_empty() {
                    self.incoming.push(Incoming::new(seg, message));
                }
                // TODO(hardint): Adjust rcv.wnd to account for received bytes

                // TODO(hardint): From the spec:
                //
                // Once the TCP endpoint takes responsibility for the data, it
                // advances RCV.NXT over the data accepted. Send an
                // acknowledgment of the form:
                // <SEQ=SND.NXT><ACK=RCV.NXT><CTL=ACK>
                //
                // Am I supposed to advance this no matter what or only if we
                // have already ACKed all the bytes that came before this
                // message? Do we only do this is the segment check succeeds? If
                // we don't generate the ACK here, where?
            }

            State::SynSent
            | State::SynReceived
            | State::CloseWait
            | State::Closing
            | State::LastAck
            | State::TimeWait => {
                // Ignore the segment text
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
        self.retransmission_queue
            .push_back(Outgoing::new(header, message));
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
                self.is_in_rcv_window(seq)
            }
        } else if RCV_WND == 0 {
            // When the receive window is zero, only ACKs are acceptible.
            false
        } else {
            self.is_in_rcv_window(seq) || self.is_in_rcv_window(seq + seg_len - 1)
        }
    }

    fn is_in_rcv_window(&self, n: u32) -> bool {
        mod_bounded(self.rcv.nxt, Leq, n, Le, self.rcv.nxt + RCV_WND as u32)
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
    mut seg: TcpHeader,
    message: Message,
    local: Ipv4Address,
    remote: Ipv4Address,
    iss: u32,
    mtu: Mtu,
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
struct ReceiveSequenceSpace {
    /// Initial receive sequence number
    irs: u32,
    /// Next sequence number expected on an incoming segment, and is the
    /// left or lower edge of the receive window
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

pub enum SendResult {
    Ok,
    ClosingConnection,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("{0}")]
    Header(#[from] BuildHeaderError),
    #[error("SEG.RST and RCV.NXT != SEG.SEQ")]
    BlindReset,
}

#[derive(Debug, Clone)]
pub struct Outgoing {
    seg: TcpHeader,
    message: Message,
    needs_retransmission: bool,
}

impl Outgoing {
    pub fn new(seg: TcpHeader, message: Message) -> Self {
        Self {
            seg,
            message,
            needs_retransmission: true,
        }
    }

    pub fn into_inner(self) -> (TcpHeader, Message) {
        (self.seg, self.message)
    }
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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Incoming {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.seg.seq == other.seg.seq {
            Ordering::Equal
        } else if mod_le(self.seg.seq, other.seg.seq) {
            // Reversing the order so the the priority queue handles messages
            // starting from lower sequence numbers
            Ordering::Greater
        } else {
            Ordering::Less
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
        let (header, message) = peer_a.outgoing().next().unwrap().into_inner();
        assert_eq!(header.seq, 100);
        assert!(header.ctl.syn());

        let mut peer_b = handle_listen(
            header,
            message,
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
        let (header, message) = peer_b.outgoing().next().unwrap().into_inner();
        assert_eq!(header.seq, 300);
        assert_eq!(header.ack, 101);
        assert!(header.ctl.syn());
        assert!(header.ctl.ack());

        peer_a.segment_arrives(header, message).unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 4
        let (header, message) = peer_a.outgoing().next().unwrap().into_inner();
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
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        assert_eq!(peer_a.state, State::SynSent);
        let a_syn = peer_a.outgoing().next().unwrap();
        assert_eq!(a_syn.seg.seq, 100);
        assert!(a_syn.seg.ctl.syn());

        // 3
        let mut peer_b = Tcb::open(PEER_B_ID, 300, 1500);
        assert_eq!(peer_b.state, State::SynSent);
        let b_syn = peer_b.outgoing().next().unwrap();
        assert_eq!(b_syn.seg.seq, 300);
        assert!(b_syn.seg.ctl.syn());

        peer_a.segment_arrives(b_syn.seg, b_syn.message).unwrap();
        assert_eq!(peer_a.state, State::SynReceived);

        // 4
        peer_b.segment_arrives(a_syn.seg, a_syn.message).unwrap();
        assert_eq!(peer_b.state, State::SynReceived);

        // 5
        let a_syn_ack = peer_a.outgoing().next().unwrap();
        assert!(a_syn_ack.seg.ctl.syn());
        assert!(a_syn_ack.seg.ctl.ack());
        assert_eq!(a_syn_ack.seg.seq, 100);
        assert_eq!(a_syn_ack.seg.ack, 301);

        // 6
        let b_syn_ack = peer_b.outgoing().next().unwrap();
        assert!(b_syn_ack.seg.ctl.syn());
        assert!(b_syn_ack.seg.ctl.ack());
        assert_eq!(b_syn_ack.seg.seq, 300);
        assert_eq!(b_syn_ack.seg.ack, 101);

        peer_a
            .segment_arrives(b_syn_ack.seg, b_syn_ack.message)
            .unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 7
        peer_b
            .segment_arrives(a_syn_ack.seg, a_syn_ack.message)
            .unwrap();
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
        let peer_a_syn = peer_a.outgoing().next().unwrap();
        assert!(peer_a_syn.seg.ctl.syn());
        assert_eq!(peer_a_syn.seg.seq, 100);

        // 3
        const GHOST_ID: ConnectionId = ConnectionId {
            local: Socket {
                address: Ipv4Address::new([123, 45, 67, 89]),
                port: 0xbabe,
            },
            remote: PEER_B_ID.local,
        };
        let mut ghost = Tcb::open(GHOST_ID, 90, 1500);
        let ghost_syn = ghost.outgoing().next().unwrap();
        assert!(ghost_syn.seg.ctl.syn());
        assert_eq!(ghost_syn.seg.seq, 90);

        let mut peer_b = handle_listen(
            ghost_syn.seg,
            ghost_syn.message,
            GHOST_ID.remote.address,
            GHOST_ID.local.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 4
        let peer_b_syn_ack = peer_b.outgoing().next().unwrap();
        assert!(peer_b_syn_ack.seg.ctl.syn());
        assert!(peer_b_syn_ack.seg.ctl.ack());
        assert_eq!(peer_b_syn_ack.seg.seq, 300);
        assert_eq!(peer_b_syn_ack.seg.ack, 91);

        peer_a
            .segment_arrives(peer_b_syn_ack.seg, peer_b_syn_ack.message)
            .unwrap();
        assert_eq!(peer_a.state, State::SynSent);

        // 5
        let peer_a_rst = peer_a.outgoing().next().unwrap();
        assert!(peer_a_rst.seg.ctl.rst());
        assert_eq!(peer_a_rst.seg.seq, 91);

        let receive_result = peer_b
            .segment_arrives(peer_a_rst.seg, peer_a_rst.message)
            .unwrap();
        assert_eq!(receive_result, ReceiveResult::ReturnToListen);

        // 6
        let mut peer_b = handle_listen(
            peer_a_syn.seg,
            peer_a_syn.message,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            400,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();

        // 7
        let peer_b_syn_ack = peer_b.outgoing().next().unwrap();
        assert!(peer_b_syn_ack.seg.ctl.syn());
        assert!(peer_b_syn_ack.seg.ctl.ack());
        assert_eq!(peer_b_syn_ack.seg.seq, 400);
        assert_eq!(peer_b_syn_ack.seg.ack, 101);

        peer_a
            .segment_arrives(peer_b_syn_ack.seg, peer_b_syn_ack.message)
            .unwrap();
        assert_eq!(peer_a.state, State::Established);

        // 8
        let peer_a_ack = peer_a.outgoing().next().unwrap();
        assert!(peer_a_ack.seg.ctl.ack());
        assert_eq!(peer_a_ack.seg.seq, 101);
        assert_eq!(peer_a_ack.seg.ack, 401);
    }

    // TODO(hardint): Add tests for the exchanges in figures 9 through 11 about
    // half-open connections

    fn established_pair() -> (Tcb, Tcb) {
        let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
        let peer_a_syn = peer_a.outgoing().next().unwrap();
        let mut peer_b = handle_listen(
            peer_a_syn.seg,
            peer_a_syn.message,
            PEER_B_ID.local.address,
            PEER_B_ID.remote.address,
            300,
            1500,
        )
        .unwrap()
        .tcb()
        .unwrap();
        let peer_b_syn_ack = peer_b.outgoing().next().unwrap();
        peer_a
            .segment_arrives(peer_b_syn_ack.seg, peer_b_syn_ack.message)
            .unwrap();
        let peer_a_ack = peer_a.outgoing().next().unwrap();
        peer_b
            .segment_arrives(peer_a_ack.seg, peer_a_ack.message)
            .unwrap();
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

        let peer_a_fin = peer_a.outgoing().next().unwrap();
        assert!(peer_a_fin.seg.ctl.fin());
        assert!(peer_a_fin.seg.ctl.ack());
        assert_eq!(peer_a_fin.seg.seq, 100);
        assert_eq!(peer_a_fin.seg.ack, 300);

        peer_b
            .segment_arrives(peer_a_fin.seg, peer_a_fin.message)
            .unwrap();
        assert_eq!(peer_b.state, State::CloseWait);

        // 3
        let peer_b_ack = peer_b.outgoing().next().unwrap();
        assert!(peer_b_ack.seg.ctl.ack());
        assert_eq!(peer_b_ack.seg.seq, 300);
        assert_eq!(peer_b_ack.seg.ack, 101);

        peer_a
            .segment_arrives(peer_b_ack.seg, peer_b_ack.message)
            .unwrap();
        assert_eq!(peer_a.state, State::FinWait2);

        // 4
        peer_b.close();
        assert_eq!(peer_b.state, State::LastAck);

        let peer_b_fin = peer_b.outgoing().next().unwrap();
        assert!(peer_b_fin.seg.ctl.fin());
        assert!(peer_b_fin.seg.ctl.ack());
        assert_eq!(peer_b_fin.seg.seq, 300);
        assert_eq!(peer_b_fin.seg.ack, 101);

        peer_a
            .segment_arrives(peer_b_fin.seg, peer_b_fin.message)
            .unwrap();
        assert_eq!(peer_a.state, State::TimeWait);

        // 5
        let peer_a_ack = peer_a.outgoing().next().unwrap();
        assert!(peer_a_ack.seg.ctl.ack());
        assert_eq!(peer_a_ack.seg.seq, 101);
        assert_eq!(peer_a_ack.seg.ack, 301);

        let receive_result = peer_b
            .segment_arrives(peer_a_ack.seg, peer_a_ack.message)
            .unwrap();
        assert_eq!(receive_result, ReceiveResult::FinalizeClose);

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
        let fin_ack_a = peer_a.outgoing().next().unwrap();
        assert_eq!(fin_ack_a.seg.seq, 100);
        assert_eq!(fin_ack_a.seg.ack, 300);
        assert!(fin_ack_a.seg.ctl.fin());
        assert!(fin_ack_a.seg.ctl.ack());

        peer_b.close();
        assert_eq!(peer_b.state, State::FinWait1);
        let fin_ack_b = peer_a.outgoing().next().unwrap();
        assert_eq!(fin_ack_b.seg.seq, 300);
        assert_eq!(fin_ack_b.seg.ack, 100);
        assert!(fin_ack_b.seg.ctl.fin());
        assert!(fin_ack_b.seg.ctl.ack());

        // 3
        peer_a
            .segment_arrives(fin_ack_b.seg, fin_ack_b.message)
            .unwrap();
        assert_eq!(peer_a.state, State::Closing);
        let ack_a = peer_a.outgoing().next().unwrap();
        assert_eq!(ack_a.seg.seq, 101);
        assert_eq!(ack_a.seg.ack, 301);
        assert!(ack_a.seg.ctl.ack());

        peer_b
            .segment_arrives(fin_ack_a.seg, fin_ack_a.message)
            .unwrap();
        assert_eq!(peer_b.state, State::Closing);
        let ack_b = peer_b.outgoing().next().unwrap();
        assert_eq!(ack_b.seg.seq, 101);
        assert_eq!(ack_b.seg.ack, 301);
        assert!(ack_b.seg.ctl.ack());

        // 4
        peer_a.segment_arrives(ack_b.seg, ack_b.message).unwrap();
        assert_eq!(peer_a.state, State::TimeWait);
        assert_eq!(
            peer_a.advance_time(MSL.mul_f32(2.1)),
            AdvanceTimeResult::CloseConnection
        );

        peer_b.segment_arrives(ack_a.seg, ack_a.message).unwrap();
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
        peer_a.send(Message::new(expected)).unwrap();
        for outgoing in peer_a.outgoing() {
            peer_b
                .segment_arrives(outgoing.seg, outgoing.message)
                .unwrap();
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
        peer_a.send(Message::new(expected.clone())).unwrap();
        let mut count = 0;
        for outgoing in peer_a.outgoing() {
            count += 1;
            peer_b
                .segment_arrives(outgoing.seg, outgoing.message)
                .unwrap();
        }
        let received = peer_b.receive();
        assert_eq!(count, 3);
        assert_eq!(expected, received);
    }

    #[test]
    fn message_retransmission() {
        let expected: Vec<_> = std::iter::repeat(0)
            .enumerate()
            .map(|(i, _)| i as u8)
            .take(8000) // This is beyond our receive window now
            .collect();
        let (mut peer_a, mut peer_b) = established_pair();
        peer_a.send(Message::new(expected.clone())).unwrap();
        let mut received = vec![];
        while received.len() != expected.len() {
            for outgoing in peer_a.outgoing() {
                peer_b
                    .segment_arrives(outgoing.seg, outgoing.message)
                    .unwrap();
            }
            received.extend(peer_b.receive());
            for outgoing in peer_b.outgoing() {
                peer_a
                    .segment_arrives(outgoing.seg, outgoing.message)
                    .unwrap();
            }
            peer_a.advance_time(Duration::from_millis(666));
            peer_b.advance_time(Duration::from_millis(666));
        }
        assert_eq!(expected, received);
    }
}
