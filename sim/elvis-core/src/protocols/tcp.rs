use super::ipv4::Ipv4Address;
use crate::{
    control::{Key, Primitive},
    protocol::{Context, ProtocolId},
    session::SharedSession,
    Control, Protocol,
};
use std::{error::Error, sync::Arc};
use thiserror::Error as ThisError;
use tokio::sync::{mpsc::Sender, Barrier};

mod tcp_parsing;
mod tcp_session;

pub struct Tcp {}

impl Tcp {
    pub const ID: ProtocolId = ProtocolId::from_string("tcp");
}

impl Protocol for Tcp {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        todo!()
    }

    fn listen(
        self: Arc<Self>,
        upstream: ProtocolId,
        participants: Control,
        context: Context,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn demux(
        self: Arc<Self>,
        message: crate::Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, Box<dyn Error>> {
        todo!()
    }
}

enum ConnectionState {
    /// Represents waiting for a connection request from any remote TCP and
    /// port.
    Listen,
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

/// Uniquely identifies one end of a connection
struct Socket {
    address: Ipv4Address,
    port: u16,
}

///            1         2          3          4
///       ----------|----------|----------|----------
///              SND.UNA    SND.NXT    SND.UNA
///                                   +SND.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers of unacknowledged data
/// 3 - sequence numbers allowed for new data transmission (send window)
/// 4 - future sequence numbers which are not yet allowed
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

///                1          2          3
///            ----------|----------|----------
///                   RCV.NXT    RCV.NXT
///                             +RCV.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers allowed for new reception
/// 3 - future sequence numbers which are not yet allowed
struct ReceiveSequenceSpace {
    /// Next
    nxt: u32,
    /// Window
    wnd: u16,
    /// Initial receive sequence
    irs: u32,
}

struct Tcb {
    local_socket: Socket,
    remote_socket: Socket,
    send: SendSequenceSpace,
    recv: ReceiveSequenceSpace,
}

fn is_between_wrapped(a: u32, b: u32, c: u32) -> bool {
    (a < b && b < c) || (c < a && a < b) || (b < c && c < a)
}

#[derive(Debug, ThisError)]
pub enum TcpError {
    #[error("Too few bytes to constitute a TCP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    InvalidChecksum { actual: u16, expected: u16 },
    #[error("Data offset was different from that expected for a simple header")]
    UnexpectedOptions,
    #[error("The TCP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}
