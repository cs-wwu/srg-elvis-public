use elvis::{
    core::{
        Control, ControlFlow, ControlKey, Message, NetworkLayer, Protocol, ProtocolContext,
        ProtocolId,
    },
    protocols::{Application, Tap, UserProcess},
};
use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    messages: Vec<Message>,
}

impl Capture {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn messages(&self) -> &[Message] {
        self.messages.as_slice()
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
    pub tap: Rc<RefCell<Tap>>,
    pub capture: Rc<RefCell<UserProcess<Capture>>>,
    pub context: ProtocolContext,
}

pub fn setup() -> Setup {
    let tap = Tap::new(vec![1500]);
    let tap = Rc::new(RefCell::new(tap));
    let capture = Rc::new(RefCell::new(UserProcess::new(Capture::new())));
    let protocols: [Rc<RefCell<dyn Protocol>>; 2] = [tap.clone(), capture.clone()];
    let protocols: HashMap<_, _> = protocols
        .into_iter()
        .map(|protocol| {
            let id = protocol.borrow().id();
            (id, protocol)
        })
        .collect();
    let context = ProtocolContext::new(Rc::new(protocols));
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
        .borrow_mut()
        .open_active(Capture::ID, tap_control(), context.clone())?;
    let message = Message::new("Hello!");
    session
        .borrow_mut()
        .send(session.clone(), message, context)?;
    let delivery = tap.borrow_mut().outgoing();
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
        capture,
        context,
    } = setup();
    tap.borrow_mut()
        .listen(Capture::ID, Control::default(), context.clone())?;
    let header: [u8; 2] = Capture::ID.into();
    let message = Message::new("Hello!").with_header(&header);
    tap.borrow_mut().accept_incoming(message, 0, context)?;
    let capture = capture.borrow();
    let application = capture.application();
    let messages = application.messages();
    assert_eq!(messages.len(), 1);
    let message = messages.iter().next().unwrap().clone();
    assert_eq!(message, Message::new("Hello!"));
    Ok(())
}
