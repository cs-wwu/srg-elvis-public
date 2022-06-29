use elvis::{
    core::{ArcProtocol, Control, ControlKey, Message, Protocol, ProtocolContext, ProtocolMap},
    protocols::{Capture, Nic},
};
use std::{
    error::Error,
    sync::{Arc, RwLock},
};

// Todo: Use a Capture as the other protocol, not a NIC

fn control() -> Control {
    let mut control = Control::default();
    control.insert(ControlKey::NetworkIndex, 0u8.into());
    control
}

fn nic() -> Nic {
    Nic::new(vec![1500])
}

fn shared_nic() -> ArcProtocol {
    Arc::new(RwLock::new(nic()))
}

#[test]
fn id() {
    assert_eq!(nic().id(), Nic::ID);
}

#[test]
fn open_active() -> Result<(), Box<dyn Error>> {
    let mut nic1 = nic();
    let nic2 = shared_nic();
    nic1.open_active(nic2, control())?;
    Ok(())
}

#[test]
#[should_panic]
fn open_passive() {
    let mut nic1 = nic();
    let nic2 = shared_nic();
    nic1.open_passive(nic2, control()).unwrap();
}

#[test]
fn demux() -> Result<(), Box<dyn Error>> {
    let mut nic1 = nic();
    let nic2 = shared_nic();
    nic1.add_demux_binding(nic2, control())?;
    let header: [u8; 2] = Nic::ID.into();
    let message = Message::new(&header);
    let context = ProtocolContext::new(ProtocolMap::default());
    nic1.demux(message, context)?;
    Ok(())
}
