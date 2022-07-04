use elvis::{
    applications::Capture,
    core::{Control, ControlKey, Message, Protocol, ProtocolContext, RcProtocol},
    protocols::{Application, Tap, UserProcess},
};
use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

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

impl Setup {
    pub fn new() -> Self {
        let tap = Tap::new_shared(vec![1500]);
        let capture = Capture::new_shared();
        let protocols: [RcProtocol; 2] = [tap.clone(), capture.clone()];
        let protocols: HashMap<_, _> = protocols
            .into_iter()
            .map(|protocol| {
                let id = protocol.borrow().id();
                (id, protocol)
            })
            .collect();
        let context = ProtocolContext::new(Rc::new(protocols));
        Self {
            tap,
            capture,
            context,
        }
    }
}

#[test]
fn open_active() -> Result<(), Box<dyn Error>> {
    let Setup {
        tap, mut context, ..
    } = Setup::new();
    let session = tap
        .borrow_mut()
        .open_active(Capture::ID, tap_control(), &mut context)?;
    let message = Message::new("Hello!");
    session
        .borrow_mut()
        .send(session.clone(), message, &mut context)?;
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
        mut context,
    } = Setup::new();
    tap.borrow_mut()
        .listen(Capture::ID, Control::default(), &mut context)?;
    let header: [u8; 2] = Capture::ID.into();
    let message = Message::new("Hello!").with_header(&header);
    tap.borrow_mut().accept_incoming(message, 0, &mut context)?;
    let capture = capture.borrow();
    assert_eq!(
        capture.application().message().unwrap(),
        Message::new("Hello!")
    );
    Ok(())
}
