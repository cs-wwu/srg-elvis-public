use elvis::{
    core::{Control, ControlKey, Protocol, ProtocolContext},
    protocols::{Capture, Nic},
};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

// Todo: Use a Capture as the other protocol, not a NIC

fn nic_control() -> Control {
    let mut control = Control::default();
    control.insert(ControlKey::NetworkIndex, 0u8.into());
    control
}

struct Setup {
    pub nic: Arc<RwLock<Nic>>,
    pub capture: Arc<RwLock<Capture>>,
    pub context: ProtocolContext,
}

fn setup() -> Setup {
    let nic = Arc::new(RwLock::new(Nic::new(vec![1500])));
    let capture = Arc::new(RwLock::new(Capture::new()));
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
    let Setup {
        nic,
        capture,
        context,
    } = setup();
    nic.write()
        .unwrap()
        .open_active(capture, nic_control(), context)?;
    Ok(())
}

#[test]
fn open_passive() -> Result<(), Box<dyn Error>> {
    let Setup {
        nic,
        capture,
        context,
    } = setup();
    capture
        .write()
        .unwrap()
        .open_passive(nic, Control::default(), context)?;
    Ok(())
}

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
