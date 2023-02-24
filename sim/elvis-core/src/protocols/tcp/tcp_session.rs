use super::tcb::{AdvanceTimeResult, Segment, SegmentArrivesResult, Tcb};
use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError, SharedSession},
    Id, Message, ProtocolMap, Session,
};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
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
    /// The transmission control block for the connection
    tcb: RwLock<Tcb>,
    /// The upstream protocol
    upstream: Id,
    /// The downstream session
    downstream: SharedSession,
}

impl TcpSession {
    /// Create a new TCP session
    pub fn new(tcb: RwLock<Tcb>, upstream: Id, downstream: SharedSession) -> Self {
        Self {
            tcb,
            upstream,
            downstream,
        }
    }

    /// Receive an incoming message from the TCP as part of the demux flow
    pub fn receive(
        self: Arc<Self>,
        segment: Segment,
        context: Context,
    ) -> Result<SegmentArrivesResult, ReceiveError> {
        let mut tcb = self.tcb.write().unwrap();
        let result = tcb.segment_arrives(segment);
        let segments = tcb.segments();
        drop(tcb);
        self.deliver_outgoing(segments, context.clone())?;
        let mut tcb = self.tcb.write().unwrap();
        let received = tcb.receive();
        drop(tcb);
        if !received.is_empty() {
            context
                .clone()
                .protocol(self.upstream)
                .ok_or(ReceiveError::Protocol(self.upstream))?
                .demux(received, self.clone(), context)?;
        }
        Ok(result)
    }

    /// Increase the current time by the given delta, used to trigger timeouts
    pub fn advance_time(
        self: Arc<Self>,
        delta_time: Duration,
        protocols: ProtocolMap,
    ) -> AdvanceTimeResult {
        let mut tcb = self.tcb.write().unwrap();
        let result = tcb.advance_time(delta_time);
        let context = Context::new(protocols);
        let segments = tcb.segments();
        drop(tcb);
        match self.deliver_outgoing(segments, context) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Send error while advancing time: {}", e);
            }
        }
        result
    }

    /// Transfer outgoing segments from the TCB to the downstream session
    fn deliver_outgoing(&self, segments: Vec<Segment>, context: Context) -> Result<(), SendError> {
        for mut segment in segments {
            segment.text.header(segment.header.serialize());
            self.downstream
                .clone()
                .send(segment.text, context.clone())?;
        }
        Ok(())
    }
}

impl Session for TcpSession {
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), SendError> {
        let mut tcb = self.tcb.write().unwrap();
        tcb.send(&message);
        let segments = tcb.segments();
        drop(tcb);
        self.deliver_outgoing(segments, context)?;
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
