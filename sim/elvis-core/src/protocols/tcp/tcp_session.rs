use super::{
    tcp_parsing::{TcpHeader, TcpHeaderBuilder},
    Iss,
};
use crate::{
    control::{Key, Primitive},
    protocol::{Context, OpenError, ProtocolId},
    protocols::ipv4::Ipv4Address,
    session::{QueryError, SendError, SharedSession},
    Message, ProtocolMap, Session,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, Notify};

// TODO(hardint): The unwraps used on channels should be removed and cleaned up
// along with proper simulation teardown.

// NOTE(hardint): This initial implementation assumes that all send calls have
// the PSH flag set, meaning that the TCP will start sending any queued messages
// as soon as they are passed in. At a later time, it should be modified such
// that the TCP waits until an MTU worth of data is available before sending
// unless the PSH flag is set. Optionally, the TCP can start delivering small
// packets that have been queued after some timeout.

pub struct TcpSession {
    tcb: Arc<RwLock<Tcb>>,
    upstream: ProtocolId,
    downstream: SharedSession,
    /// Messages to be sent are queued here for delivery on a separate thread.
    send_queue: Arc<mpsc::Sender<Message>>,
    receive_queue: Arc<mpsc::Sender<Message>>,
}

impl TcpSession {
    /// Open a new connection. See 3.10.1.
    pub fn open(
        id: SessionId,
        upstream: ProtocolId,
        downstream: SharedSession,
        iss: Iss,
        protocols: ProtocolMap,
    ) -> Result<Arc<Self>, OpenError> {
        const WND: u16 = 4096;
        let (send_queue_sender, send_queue_receiver) = mpsc::channel(16);
        let (receive_queue_sender, receive_queue_receiver) = mpsc::channel(16);
        let session = Arc::new(Self {
            upstream,
            downstream: downstream.clone(),
            send_queue: Arc::new(send_queue_sender),
            receive_queue: Arc::new(receive_queue_sender),
            tcb: Arc::new(RwLock::new(Tcb {
                state: State::SynSent,
                id,
                send: SendSequenceSpace::new(iss, WND),
                recv: Default::default(),
            })),
        });

        let established_notify = Arc::new(Notify::new());
        {
            let session = session.clone();
            let established_notify = established_notify.clone();
            tokio::spawn(async move {
                session
                    .poll_send_queue(established_notify, send_queue_receiver)
                    .await
            });
        }

        {
            let session = session.clone();
            tokio::spawn(async move {
                session
                    .poll_receive_queue(established_notify, receive_queue_receiver)
                    .await;
            });
        }

        let header = TcpHeaderBuilder::new(id, iss.into(), WND)
            .syn()
            .build([].into_iter())
            .map_err(|_| SendError::Other)?;
        let message = Message::new(header);
        let context = Context::new(protocols);
        downstream
            .clone()
            .send(message, context.clone())
            .map_err(|_| SendError::Other)?;

        Ok(session)
    }

    async fn poll_send_queue(
        self: Arc<Self>,
        established_notify: Arc<Notify>,
        mut send_queue: mpsc::Receiver<Message>,
    ) {
        // Don't start sending until the connection has been established
        established_notify.notified().await;

        while let Some(_message) = send_queue.recv().await {
            todo!()
        }
    }

    // See 3.10.7
    async fn poll_receive_queue(
        self: Arc<Self>,
        established_notify: Arc<Notify>,
        mut receive_queue: mpsc::Receiver<Message>,
    ) {
        // This is the logic for receive, not send
        while let Some(message) = receive_queue.recv().await {
            let (src_address, dst_address, state) = {
                let tcb = self.tcb.read().unwrap();
                (tcb.id.src.address, tcb.id.dst.address, tcb.state)
            };

            let header = match TcpHeader::from_bytes(message.iter(), src_address, dst_address) {
                Ok(header) => header,
                Err(e) => {
                    tracing::error!("Failed to parse TCP header: {}", e);
                    continue;
                }
            };

            use State::*;
            match state {
                SynSent => todo!(),
                SynReceived => todo!(),
                Established => todo!(),
                FinWait1 => todo!(),
                FinWait2 => todo!(),
                CloseWait => todo!(),
                Closing => todo!(),
                LastAck => todo!(),
                TimeWait => todo!(),
            }
            // TODO(hardint): Queue receives unless pushed.
            // Also need to perform reordering.
            // Also need to send ACKs.
        }
    }

    pub fn receive(
        self: Arc<Self>,
        message: Message,
        header: TcpHeader,
    ) -> Result<(), ReceiveError> {
        let receive_queue = self.receive_queue.clone();
        tokio::spawn(async move {
            match receive_queue.send(message).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to receive a TCP message: {}", e);
                }
            }
        });
        Ok(())
    }
}

impl Session for TcpSession {
    // See 3.10.2
    fn send(self: Arc<Self>, message: Message, _context: Context) -> Result<(), SendError> {
        let state = self.tcb.read().unwrap().state;
        use State::*;
        match state {
            SynSent | SynReceived | Established | CloseWait => {
                let send_queue = self.send_queue.clone();
                tokio::spawn(async move {
                    send_queue.send(message).await.unwrap();
                });
                Ok(())
            }
            FinWait1 | FinWait2 | Closing | LastAck | TimeWait => {
                tracing::error!("Connection closing");
                Err(SendError::Other)
            }
        }
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

//      1         2          3          4
// ----------|----------|----------|----------
//        SND.UNA    SND.NXT    SND.UNA
//                             +SND.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers of unacknowledged data
// 3 - sequence numbers allowed for new data transmission (send window)
// 4 - future sequence numbers which are not yet allowed
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

//     1          2          3
// ----------|----------|----------
//        RCV.NXT    RCV.NXT
//                  +RCV.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers allowed for new reception
// 3 - future sequence numbers which are not yet allowed
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
    state: State,
    id: SessionId,
    send: SendSequenceSpace,
    recv: ReceiveSequenceSpace,
}

/// Is `b` between `a` and `c` when accounting for modular arithmetic?
fn is_between_wrapped(a: u32, b: u32, c: u32) -> bool {
    (a < b && b < c) || (c < a && a < b) || (b < c && c < a)
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Attempted to receive on a closing connection")]
    Closing,
}
