use super::{
    tcb::{Segment, SegmentArrivesResult, Tcb},
    TcpMonitors,
};
use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError, SharedProtocol},
    protocols::tcp::tcb::AdvanceTimeResult,
    session::{QueryError, SendError, SharedSession},
    Id, Message, ProtocolMap, Session,
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::mpsc::{channel, Sender},
    time::sleep,
};

// TODO(hardint): The unwraps used on channels should be removed and cleaned up
// along with proper simulation teardown.

// NOTE(hardint): This initial implementation assumes that all send calls have
// the PSH flag set, meaning that the TCP will start sending any queued messages
// as soon as they are passed in. At a later time, it should be modified such
// that the TCP waits until an MTU worth of data is available before sending
// unless the PSH flag is set. Optionally, the TCP can start delivering small
// packets that have been queued after some timeout.

/// The session part of the TCP protocol.
pub struct TcpSession {
    send: Sender<Instruction>,
    downstream: SharedSession,
}

impl TcpSession {
    /// Create a new TCP session
    pub fn new(
        mut tcb: Tcb,
        upstream: SharedProtocol,
        downstream: SharedSession,
        protocols: ProtocolMap,
        monitors: TcpMonitors,
    ) -> Arc<Self> {
        let (send, mut recv) = channel(8);
        let me = Arc::new(Self {
            send,
            downstream: downstream.clone(),
        });
        let out = me.clone();
        let context = Context::new(protocols);
        tokio::spawn(monitors.outer.instrument(async move {
            loop {
                const TIMEOUT: Duration = Duration::from_millis(5);
                // TODO(hardint): Listen for shutdown
                select! {
                    instruction = recv.recv() => {
                        match instruction {
                            Some(instruction) => {
                                match instruction {
                                    Instruction::Incoming(segment) => {
                                        match tcb.segment_arrives(segment) {
                                            SegmentArrivesResult::Ok => {}
                                            // TODO(hardint): Signal close
                                            SegmentArrivesResult::Close => break,
                                        }
                                    }
                                    Instruction::Outgoing(message) => {
                                        tcb.send(message);
                                    }
                                }
                            }
                            // TODO(hardint): Signal close
                            None => break,
                        }
                    }
                    _ = sleep(TIMEOUT) => {
                        match tcb.advance_time(TIMEOUT) {
                            AdvanceTimeResult::Ignore => {}
                            // TODO(hardint): Signal close
                            AdvanceTimeResult::CloseConnection => break,
                        };
                    }
                };

                let segments = tcb.segments();
                let received = tcb.receive();
                let downstream = downstream.clone();
                let context = context.clone();
                let upstream = upstream.clone();
                let me = me.clone();
                tokio::spawn(monitors.inner.instrument(async move {
                    for mut segment in segments {
                        segment.text.header(segment.header.serialize());
                        match downstream.clone().send(segment.text, context.clone()).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Send error: {}", e),
                        }
                    }

                    if !received.is_empty() {
                        match upstream.demux(received, me, context).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("Demux error: {}", e),
                        }
                    }
                }));
            }
        }));
        out
    }

    /// Receive an incoming message from the TCP as part of the demux flow
    pub fn receive(self: Arc<Self>, segment: Segment, _context: Context) {
        tokio::spawn(async move {
            match self.send.send(Instruction::Incoming(segment)).await {
                Ok(_) => {}
                Err(e) => eprintln!("TCP receive error: {}", e),
            }
        });
    }
}

#[async_trait]
impl Session for TcpSession {
    async fn send(self: Arc<Self>, message: Message, _context: Context) -> Result<(), SendError> {
        tokio::spawn(async move {
            match self.send.send(Instruction::Outgoing(message)).await {
                Ok(_) => {}
                Err(e) => eprintln!("TCP send error: {}", e),
            }
        });
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        // TODO(hardint): Add queries
        self.downstream.clone().query(key)
    }
}

/// An error that occurred during `TcpSession::receive`
#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Attempted to receive on a closing connection")]
    Closing,
    #[error("Could not get a protocol for the ID {0}")]
    Protocol(Id),
    #[error("{0}")]
    Demux(#[from] DemuxError),
    #[error("{0}")]
    Send(#[from] SendError),
}

enum Instruction {
    Incoming(Segment),
    Outgoing(Message),
}
