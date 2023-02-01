use super::{
    tcb::{self, ReceiveResult, Tcb},
    tcp_parsing::TcpHeader,
};
use crate::{
    control::{Key, Primitive},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError, SharedSession},
    Id, Message, ProtocolMap, Session,
};
use std::{
    sync::{Arc, RwLock, RwLockWriteGuard},
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

pub struct TcpSession {
    tcb: RwLock<Tcb>,
    upstream: Id,
    downstream: SharedSession,
}

impl TcpSession {
    pub fn new(tcb: RwLock<Tcb>, upstream: Id, downstream: SharedSession) -> Self {
        Self {
            tcb,
            upstream,
            downstream,
        }
    }

    pub fn receive(
        self: Arc<Self>,
        seg: TcpHeader,
        message: Message,
        context: Context,
    ) -> Result<ReceiveResult, ReceiveError> {
        let mut tcb = self.tcb.write().unwrap();
        match tcb.segment_arrives(seg, message) {
            Ok(result) => {
                self.deliver_outgoing(&mut tcb, context.clone())?;
                let received = tcb.receive();
                if !received.is_empty() {
                    context
                        .clone()
                        .protocol(self.upstream)
                        .ok_or(ReceiveError::Protocol(self.upstream))?
                        .demux(Message::new(received), self.clone(), context)?;
                }
                Ok(result)
            }
            Err(e) => {
                tracing::error!("Failed to handle arriving segment: {0}", e);
                Err(e)?
            }
        }
    }

    pub fn advance_time(self: Arc<Self>, delta_time: Duration, protocols: ProtocolMap) {
        let mut tcb = self.tcb.write().unwrap();
        tcb.advance_time(delta_time);
        let context = Context::new(protocols);
        match self.deliver_outgoing(&mut tcb, context) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Send error while advancing time: {}", e);
            }
        }
    }

    fn deliver_outgoing(
        &self,
        tcb: &mut RwLockWriteGuard<Tcb>,
        context: Context,
    ) -> Result<(), SendError> {
        for (seg, mut message) in tcb.outgoing() {
            message.prepend(seg.serialize());
            self.downstream.clone().send(message, context.clone())?;
        }
        Ok(())
    }
}

impl Session for TcpSession {
    // See 3.10.2
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), SendError> {
        let mut tcb = self.tcb.write().unwrap();
        tcb.send(message).map_err(|_| SendError::Header)?;
        self.deliver_outgoing(&mut tcb, context)?;
        Ok(())
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
    #[error("{0}")]
    Tcb(#[from] tcb::ReceiveError),
    #[error("Could not get a protocol for the ID {0}")]
    Protocol(Id),
    #[error("{0}")]
    Demux(#[from] DemuxError),
    #[error("{0}")]
    Send(#[from] SendError),
}
