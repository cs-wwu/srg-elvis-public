use elvis::{
    core::{
        Control, ControlFlow, ControlKey, Message, NetworkLayer, Protocol, ProtocolContext,
        ProtocolId,
    },
    protocols::{Application, Tap, UserProcess},
};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    messages: Vec<Message>,
}

impl Capture {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

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

pub fn tap_control() -> Control {
    let mut control = Control::default();
    control.insert(ControlKey::NetworkIndex, 0u8);
    control
}
pub struct Setup {
    pub tap: Arc<RwLock<Tap>>,
    pub capture: Arc<RwLock<UserProcess<Capture>>>,
    pub context: ProtocolContext,
}

pub fn setup() -> Setup {
    let mut tap = Tap::new(vec![1500]);
    let tap_session = tap
        .open_active(Capture::ID, tap_control(), ProtocolContext::default())
        .unwrap();
    let tap = Arc::new(RwLock::new(tap));
    let capture = Arc::new(RwLock::new(UserProcess::new(Capture::new())));
    let protocols: [Arc<RwLock<dyn Protocol>>; 2] = [tap.clone(), capture.clone()];
    let protocols: HashMap<_, _> = protocols
        .into_iter()
        .map(|protocol| {
            let id = protocol.read().unwrap().id();
            (id, protocol)
        })
        .collect();
    let context = ProtocolContext::new(Arc::new(protocols));
    Setup {
        tap,
        capture,
        context,
    }
}

#[test]
fn open_active() -> Result<(), Box<dyn Error>> {
    let Setup { tap, context, .. } = setup();
    let session = tap
        .write()
        .unwrap()
        .open_active(Capture::ID, tap_control(), context.clone())?;
    let message = Message::new("Hello!");
    session
        .write()
        .unwrap()
        .send(session.clone(), message, context)?;
    let delivery = tap.write().unwrap().outgoing();
    assert_eq!(delivery.len(), 1);
    let delivery = delivery.into_iter().next().unwrap();
    assert_eq!(delivery.0, 0);
    assert_eq!(delivery.1.len(), 1);
    let delivery = delivery.1.into_iter().next().unwrap();
    assert_eq!(delivery, Message::new("\x04\x00Hello!"));
    Ok(())
}

#[test]
fn tap_receive() -> Result<(), Box<dyn Error>> {
    let Setup {
        tap,
        capture: _,
        context,
    } = setup();
    tap.write()
        .unwrap()
        .listen(Capture::ID, Control::default(), context.clone())?;
    let header: [u8; 2] = Capture::ID.into();
    let message = Message::new("Hello!").with_header(&header);
    tap.write().unwrap().accept_incoming(message, 0, context)?;
    // Todo: Add back the check that the right message came in.
    Ok(())
}
