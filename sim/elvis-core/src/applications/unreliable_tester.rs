use crate::{
    core::{
        protocol::{Context, ProtocolId},
        Control, Message,
    },
    protocols::{
        ipv4::{LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort},
        user_process::Application,
        Udp, UserProcess,
    },
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc::Sender;

pub struct UnreliableTester {
    shutdown: Arc<Mutex<Option<Sender<()>>>>,
    last_receipt: Arc<Mutex<SystemTime>>,
    receipt_count: Arc<Mutex<u16>>,
}

impl UnreliableTester {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new())
    }

    pub fn receipt_count(self: Arc<Self>) -> u16 {
        *self.receipt_count.lock().unwrap()
    }
}

impl Default for UnreliableTester {
    fn default() -> Self {
        Self {
            shutdown: Default::default(),
            last_receipt: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
            receipt_count: Default::default(),
        }
    }
}

impl Application for UnreliableTester {
    const ID: ProtocolId = ProtocolId::from_string("Unreliable tester");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        *self.shutdown.lock().unwrap() = Some(shutdown);
        *self.last_receipt.lock().unwrap() = SystemTime::now();
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, [0, 0, 0, 0].into());
        RemoteAddress::set(&mut participants, [0, 0, 0, 1].into());
        LocalPort::set(&mut participants, 0xdead);
        RemotePort::set(&mut participants, 0xdead);
        let udp = context.protocol(Udp::ID).expect("No such protocol");
        udp.clone()
            .listen(Self::ID, participants.clone(), context.clone())?;
        let send_session = udp.clone().open(Self::ID, participants, context.clone())?;
        for i in 0..100u32 {
            send_session
                .clone()
                .send(Message::new(&i.to_be_bytes()), context.clone())?;
        }
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let now = SystemTime::now();
                let then = *self.last_receipt.lock().unwrap();
                let duration = now.duration_since(then).unwrap();
                if duration > Duration::from_millis(125) {
                    tokio::spawn(async move {
                        let shutdown = self.shutdown.lock().unwrap().as_ref().unwrap().clone();
                        shutdown.send(()).await.unwrap()
                    });
                    break;
                }
            }
        });
        Ok(())
    }

    fn recv(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), Box<dyn Error>> {
        *self.last_receipt.lock().unwrap() = SystemTime::now();
        *self.receipt_count.lock().unwrap() += 1;
        Ok(())
    }
}
