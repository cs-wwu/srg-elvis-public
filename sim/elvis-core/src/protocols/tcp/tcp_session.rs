use super::{tcp_parsing::TcpHeaderBuilder, TcpError};
use crate::{
    control::{Key, Primitive},
    protocol::Context,
    protocols::ipv4::Ipv4Address,
    session::SharedSession,
    Message, Session,
};
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use std::{error::Error, sync::Arc};

pub struct TcpSession {
    state: State,
    tcb: Tcb,
    downstream: SharedSession,
}

/// The initial send sequence of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Iss {
    #[default]
    Random,
    FromSeed(u64),
}

impl From<Iss> for u32 {
    fn from(iss: Iss) -> Self {
        let mut rng = match iss {
            Iss::Random => SmallRng::from_entropy(),
            Iss::FromSeed(c) => SmallRng::seed_from_u64(c),
        };
        rng.next_u32()
    }
}

impl TcpSession {
    /// Open a new connection. See 3.10.1.
    pub fn open(
        context: Context,
        id: SessionId,
        downstream: SharedSession,
        iss: Iss,
    ) -> Result<Self, TcpError> {
        let send = SendSequenceSpace::new(iss, 0x1000);

        let header = TcpHeaderBuilder::new(id, send.iss, send.wnd)
            .syn()
            .build([].into_iter())?;
        let message = Message::new(header);
        downstream
            .clone()
            .send(message, context)
            .map_err(|_| TcpError::Send)?;

        Ok(Self {
            state: State::SynSent,
            downstream,
            tcb: Tcb {
                id,
                send,
                recv: Default::default(),
            },
        })
    }
}

impl Session for TcpSession {
    fn send(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, Box<dyn Error>> {
        // TODO(hardint): Add queries
        self.downstream.clone().query(key)
    }
}

/// Uniquely identifies one end of a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Socket {
    pub address: Ipv4Address,
    pub port: u16,
}

impl Socket {
    pub fn new(address: Ipv4Address, port: u16) -> Self {
        Self { address, port }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId {
    pub src: Socket,
    pub dst: Socket,
}

impl SessionId {
    pub fn new(src: Socket, dst: Socket) -> Self {
        Self { src, dst }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum State {
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

///      1         2          3          4
/// ----------|----------|----------|----------
///        SND.UNA    SND.NXT    SND.UNA
///                             +SND.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers of unacknowledged data
/// 3 - sequence numbers allowed for new data transmission (send window)
/// 4 - future sequence numbers which are not yet allowed
#[derive(Debug, PartialEq, Eq, Hash)]
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

///     1          2          3
/// ----------|----------|----------
///        RCV.NXT    RCV.NXT
///                  +RCV.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers allowed for new reception
/// 3 - future sequence numbers which are not yet allowed
#[derive(Debug, Default, PartialEq, Eq, Hash)]
struct ReceiveSequenceSpace {
    /// Next
    nxt: u32,
    /// Window
    wnd: u16,
    /// Initial receive sequence
    irs: u32,
}

impl ReceiveSequenceSpace {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Tcb {
    id: SessionId,
    send: SendSequenceSpace,
    recv: ReceiveSequenceSpace,
}

/// Is `b` between `a` and `c` when accounting for modular arithmetic?
fn is_between_wrapped(a: u32, b: u32, c: u32) -> bool {
    (a < b && b < c) || (c < a && a < b) || (b < c && c < a)
}
