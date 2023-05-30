use super::tcb::{Segment, SegmentArrivesResult, Tcb};
use crate::{
    machine::ProtocolMap,
    protocol::DemuxError,
    protocols::tcp::tcb::{AdvanceTimeResult, State},
    session::SendError,
    Control, Message, Protocol, Session,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc::{channel, error::TryRecvError, Sender},
    time::timeout,
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
}

impl TcpSession {
    /// Create a new TCP session
    pub fn new(
        mut tcb: Tcb,
        upstream: Arc<dyn Protocol>,
        downstream: Arc<dyn Session>,
        protocols: ProtocolMap,
    ) -> Arc<Self> {
        let (send, mut recv) = channel(8);
        let me = Arc::new(Self { send });
        let out = me.clone();
        tokio::spawn(async move {
            let mut connected = false;
            'outer: loop {
                const TIMEOUT: Duration = Duration::from_millis(5);

                // This is for optimization. Tokio was spending a lot of time
                // getting the current time for timeouts, so we first process
                // any ready instructions without setting up a timeout and then
                // maybe do the timeout if there were no instructions ready.
                let mut needs_timeout = true;

                if !connected && tcb.status() == State::Established {
                    connected = true;
                    //TODO(giddinl2): upstream.notify(NotifyType::NewConnection, me.clone(), me.control);
                }

                loop {
                    match recv.try_recv() {
                        Ok(instruction) => {
                            match handle_instruction(instruction, &mut tcb) {
                                InstructionResult::Ok => {}
                                InstructionResult::Close => break 'outer,
                            }
                            needs_timeout = false;
                        }
                        Err(e) => match e {
                            TryRecvError::Empty => break,
                            TryRecvError::Disconnected => break 'outer,
                        },
                    }
                }

                if needs_timeout {
                    match timeout(TIMEOUT, recv.recv()).await {
                        Ok(instruction) => {
                            match instruction {
                                Some(instruction) => {
                                    match handle_instruction(instruction, &mut tcb) {
                                        InstructionResult::Ok => {}
                                        InstructionResult::Close => break,
                                    }
                                }
                                // TODO(hardint): Signal close
                                None => break,
                            }
                        }
                        Err(_) => {
                            match tcb.advance_time(TIMEOUT) {
                                AdvanceTimeResult::Ignore => {}
                                // TODO(hardint): Signal close
                                AdvanceTimeResult::CloseConnection => break,
                            };
                        }
                    }
                }

                for mut segment in tcb.segments() {
                    segment.text.header(segment.header.serialize());
                    match downstream.send(segment.text, protocols.clone()) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Send error: {}", e),
                    }
                }

                let received = tcb.receive();
                if !received.is_empty() {
                    match upstream.demux(received, me.clone(), Control::new(), protocols.clone()) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Demux error: {}", e),
                    }
                }
            }
        });
        out
    }

    /// Receive an incoming message from the TCP as part of the demux flow
    pub fn receive(&self, segment: Segment) {
        let send = self.send.clone();
        tokio::spawn(async move {
            match send.send(Instruction::Incoming(segment)).await {
                Ok(_) => {}
                Err(e) => eprintln!("TCP receive error: {}", e),
            }
        });
    }
}

fn handle_instruction(instruction: Instruction, tcb: &mut Tcb) -> InstructionResult {
    match instruction {
        Instruction::Incoming(segment) => match tcb.segment_arrives(segment) {
            SegmentArrivesResult::Ok => InstructionResult::Ok,
            SegmentArrivesResult::Close => InstructionResult::Close,
        },
        Instruction::Outgoing(message) => {
            tcb.send(message);
            InstructionResult::Ok
        }
    }
}

enum InstructionResult {
    Ok,
    Close,
}

impl Session for TcpSession {
    fn send(&self, message: Message, _protocols: ProtocolMap) -> Result<(), SendError> {
        let send = self.send.clone();
        tokio::spawn(async move {
            match send.send(Instruction::Outgoing(message)).await {
                Ok(_) => {}
                Err(e) => eprintln!("TCP send error: {}", e),
            }
        });
        Ok(())
    }
}

/// An error that occurred during `TcpSession::receive`
#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("{0}")]
    Demux(#[from] DemuxError),
    #[error("{0}")]
    Send(#[from] SendError),
}

enum Instruction {
    Incoming(Segment),
    Outgoing(Message),
}
