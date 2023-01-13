use super::tcb::Tcb;
use crate::{
    control::{Key, Primitive},
    protocol::{Context, ProtocolId},
    session::{QueryError, SendError, SharedSession},
    Message, Session,
};
use std::sync::{Arc, RwLock};

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
