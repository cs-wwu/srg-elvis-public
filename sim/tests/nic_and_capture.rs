use elvis::{
    core::{Control, ControlFlow, ControlKey, Message, Protocol, ProtocolContext},
    protocols::{Application, Nic, UserProcess},
};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
struct Capture {
    messages: Vec<Message>,
}

impl Capture {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Application for Capture {
    fn awake(&mut self, _context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        Ok(if self.messages.is_empty() {
            ControlFlow::Continue
        } else {
            ControlFlow::EndSimulation
        })
    }

    fn recv(&mut self, message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        self.messages.push(message);
        Ok(())
    }
}

pub fn nic_control() -> Control {
    let mut control = Control::default();
    control.insert(ControlKey::NetworkIndex, 0u8);
    control
}
pub struct Setup {
    pub nic: Arc<RwLock<Nic>>,
    pub capture: Arc<RwLock<UserProcess>>,
    pub context: ProtocolContext,
}

pub fn setup() -> Setup {
    let mut nic = Nic::new(vec![1500]);
    let nic_session = nic
        .open_active(UserProcess::ID, nic_control(), ProtocolContext::default())
        .unwrap();
    let nic = Arc::new(RwLock::new(nic));
    let capture = Arc::new(RwLock::new(UserProcess::new(Box::new(Capture::new()))));
    let protocols: [Arc<RwLock<dyn Protocol>>; 2] = [nic.clone(), capture.clone()];
    let protocols: HashMap<_, _> = protocols
        .into_iter()
        .map(|protocol| {
            let id = protocol.read().unwrap().id();
            (id, protocol)
        })
        .collect();
    let context = ProtocolContext::new(Arc::new(protocols));
    Setup {
        nic,
        capture,
        context,
    }
}

#[test]
fn open_active() -> Result<(), Box<dyn Error>> {
    let Setup { nic, context, .. } = setup();
    let session =
        nic.write()
            .unwrap()
            .open_active(UserProcess::ID, nic_control(), context.clone())?;
    let message = Message::new("Hello!");
    session
        .write()
        .unwrap()
        .send(session.clone(), message, context)?;
    let delivery = nic.write().unwrap().outgoing();
    assert_eq!(delivery.len(), 1);
    let delivery = delivery.into_iter().next().unwrap();
    assert_eq!(delivery.0, 0);
    assert_eq!(delivery.1.len(), 1);
    let delivery = delivery.1.into_iter().next().unwrap();
    assert_eq!(delivery, Message::new("\x04\x00Hello!"));
    Ok(())
}

#[test]
fn nic_receive() -> Result<(), Box<dyn Error>> {
    let Setup {
        nic,
        capture: _,
        context,
    } = setup();
    nic.write()
        .unwrap()
        .listen(UserProcess::ID, Control::default(), context.clone())?;
    let header: [u8; 2] = UserProcess::ID.into();
    let message = Message::new("Hello!").with_header(&header);
    nic.write().unwrap().accept_incoming(message, 0, context)?;
    // Todo: Add back the check that the right message came in.
    Ok(())
}
