use super::tcb::{Segment, SegmentArrivesResult, Tcb};
use crate::{
    control::{Key, Primitive},
    gcd::{self},
    protocol::{DemuxError, SharedProtocol},
    session::{QueryError, SendError, SharedSession},
    Control, Id, Message, Session,
};
use std::{
    sync::{Arc, RwLock, Weak},
    time::{Duration, Instant},
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
    me: RwLock<Option<Weak<dyn Session + Send + Sync + 'static>>>,
    tcb: RwLock<Tcb>,
    last_time_update: RwLock<Instant>,
    upstream: SharedProtocol,
    downstream: SharedSession,
}

impl TcpSession {
    /// Create a new TCP session
    pub fn new(tcb: Tcb, upstream: SharedProtocol, downstream: SharedSession) -> Arc<Self> {
        let me = Arc::new(Self {
            me: Default::default(),
            tcb: RwLock::new(tcb),
            last_time_update: RwLock::new(Instant::now()),
            upstream,
            downstream,
        });
        // TODO(hardint): This is disgusting. How can we avoid having to hold a
        // reference to self?
        *me.me.write().unwrap() = Some(Arc::downgrade(&(me.clone() as SharedSession)));
        {
            let me = me.clone();
            const TIMEOUT: Duration = Duration::from_millis(100);
            // TODO(hardint): This job needs to repeat
            gcd::job_at(move || {
                let now = Instant::now();
                let mut lock = me.last_time_update.write().unwrap();
                if now > *lock + TIMEOUT {
                    *lock = now;
                    drop(lock);
                    let mut lock = me.tcb.write().unwrap();
                    // TODO(hardint): Do something with this result
                    let _ = lock.advance_time(TIMEOUT);
                    me.follow_up(&mut *lock);
                }
            });
        }
        me
    }

    /// Receive an incoming message from the TCP as part of the demux flow
    pub fn receive(&self, segment: Segment) -> SegmentArrivesResult {
        let mut tcb = self.tcb.write().unwrap();
        let result = tcb.segment_arrives(segment);
        match result {
            SegmentArrivesResult::Ok => self.follow_up(&mut *tcb),
            SegmentArrivesResult::Close => {}
        }
        result
    }

    fn me(&self) -> SharedSession {
        Weak::upgrade(&self.me.read().unwrap().as_ref().unwrap()).unwrap()
    }

    // TODO(hardint): This context might not be right. The Control should
    // probably be empty.
    fn follow_up(&self, tcb: &mut Tcb) {
        let control = Control::new();
        for mut segment in tcb.segments() {
            segment.text.header(segment.header.serialize());
            match self.downstream.send(segment.text, control.clone()) {
                Ok(_) => {}
                Err(e) => eprintln!("Send error: {}", e),
            }
        }

        let received = tcb.receive();
        if !received.is_empty() {
            match self.upstream.demux(received, self.me(), control) {
                Ok(_) => {}
                Err(e) => eprintln!("Demux error: {}", e),
            }
        }
    }
}

impl Session for TcpSession {
    fn send(&self, message: Message, _control: Control) -> Result<(), SendError> {
        let mut tcb = self.tcb.write().unwrap();
        tcb.send(message);
        self.follow_up(&mut *tcb);
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        // TODO(hardint): Add queries
        self.downstream.query(key)
    }
}

/// An error that occurred during `TcpSession::receive`
#[allow(unused)]
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
