use elvis::{
    core::{Control, ControlKey, Message, Protocol, ProtocolContext},
    protocols::{Capture, Nic},
};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

pub fn nic_control() -> Control {
    let mut control = Control::default();
    control.insert(ControlKey::NetworkIndex, 0u8.into());
    control
}

pub struct Setup {
    pub nic: Arc<RwLock<Nic>>,
    pub capture: Arc<RwLock<Capture>>,
    pub context: ProtocolContext,
}

pub fn setup() -> Setup {
    let mut nic = Nic::new(vec![1500]);
    let nic_session = nic
        .open_active(Capture::ID, nic_control(), ProtocolContext::default())
        .unwrap();
    let nic = Arc::new(RwLock::new(nic));
    let capture = Arc::new(RwLock::new(Capture::new(nic_session)));
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
    let session = nic
        .write()
        .unwrap()
        .open_active(Capture::ID, nic_control(), context.clone())?;
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
        capture,
        context,
    } = setup();
    nic.write()
        .unwrap()
        .listen(Capture::ID, Control::default(), context.clone())?;
    let header: [u8; 2] = Capture::ID.into();
    let message = Message::new("Hello!").with_header(&header);
    nic.write().unwrap().accept_incoming(message, 0, context)?;
    let messages = capture.write().unwrap().messages();
    assert_eq!(messages.len(), 1);
    let message = messages.into_iter().next().unwrap();
    assert_eq!(message, Message::new("Hello!"));
    Ok(())
}
