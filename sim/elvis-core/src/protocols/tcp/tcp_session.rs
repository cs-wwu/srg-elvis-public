use super::{
    tcb::Tcb,
    tcp_parsing::{TcpHeader, TcpHeaderBuilder},
    ConnectionId,
};
use crate::{
    control::{Key, Primitive},
    protocol::{Context, OpenError, ProtocolId},
    protocols::tcp::tcb::{SendSequenceSpace, State},
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
        id: ConnectionId,
        upstream: ProtocolId,
        downstream: SharedSession,
        iss: u32,
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
                snd: SendSequenceSpace::new(iss, WND),
                rcv: Default::default(),
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
        _established_notify: Arc<Notify>,
        mut receive_queue: mpsc::Receiver<Message>,
    ) {
        // This is the logic for receive, not send
        while let Some(message) = receive_queue.recv().await {
            let (src_address, dst_address) = {
                let tcb = self.tcb.read().unwrap();
                (tcb.id.src.address, tcb.id.dst.address)
            };

            let _header = match TcpHeader::from_bytes(message.iter(), src_address, dst_address) {
                Ok(header) => header,
                Err(e) => {
                    tracing::error!("Failed to parse TCP header: {}", e);
                    continue;
                }
            };

            // TODO(hardint): Queue receives unless pushed.
            // Also need to perform reordering.
            // Also need to send ACKs.
        }
    }

    pub fn receive(
        self: Arc<Self>,
        message: Message,
        _header: TcpHeader,
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
    fn send(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), SendError> {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        // TODO(hardint): Add queries
        self.downstream.clone().query(key)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Attempted to receive on a closing connection")]
    Closing,
}
