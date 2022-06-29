use elvis::{
    core::{ArcProtocol, Control, ControlKey, Protocol, ProtocolContext},
    protocols::{Capture, Nic},
};
use std::{
    collections::HashMap,
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

fn shared_capture() -> ArcProtocol {
    Arc::new(RwLock::new(Capture::new()))
}

#[test]
fn id() {
    assert_eq!(nic().id(), Nic::ID);
}

#[test]
fn open_active() -> Result<(), Box<dyn Error>> {
    let nic = shared_nic();
    let capture = shared_capture();
    let protocols: [Arc<RwLock<dyn Protocol>>; 2] = [nic.clone(), capture.clone()];
    let protocols: HashMap<_, _> = protocols
        .into_iter()
        .map(|protocol| {
            let id = protocol.read().unwrap().id();
            (id, protocol)
        })
        .collect();
    let context = ProtocolContext::new(Arc::new(protocols));
    nic.write()
        .unwrap()
        .open_active(capture, control(), context)?;
    Ok(())
}

// #[test]
// #[should_panic]
// fn open_passive() {
//     let mut nic1 = nic();
//     let nic2 = shared_nic();
//     nic1.open_passive(nic2, control()).unwrap();
// }

// #[test]
// fn demux() -> Result<(), Box<dyn Error>> {
//     let mut nic1 = nic();
//     let nic2 = shared_nic();
//     nic1.add_demux_binding(nic2, control())?;
//     let header: [u8; 2] = Nic::ID.into();
//     let message = Message::new(&header);
//     let context = ProtocolContext::new(ProtocolMap::default());
//     nic1.demux(message, context)?;
//     Ok(())
// }
