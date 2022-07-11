use crate::core::Control;
use thiserror::Error as ThisError;

static LOCAL_PORT_KEY: &str = "udp_local_port";
static REMOTE_PORT_KEY: &str = "udp_remote_port";

pub fn set_local_port(control: &mut Control, port: u16) {
    control.insert(LOCAL_PORT_KEY, port)
}

pub fn set_remote_port(control: &mut Control, port: u16) {
    control.insert(REMOTE_PORT_KEY, port)
}

pub fn get_local_port(control: &Control) -> u16 {
    control
        .get(LOCAL_PORT_KEY)
        .expect("Missing local port")
        .to_u16()
        .expect("Incorrect local port type")
}

pub fn get_remote_port(control: &Control) -> u16 {
    control
        .get(REMOTE_PORT_KEY)
        .expect("Missing remote port")
        .to_u16()
        .expect("Incorrect remote port type")
}

#[derive(Debug, ThisError)]
pub enum UdpError {
    #[error("Tried to create an existing session")]
    SessionExists,
    #[error("Tried to demux with a missing session and no listen bindings")]
    MissingSession,
}
