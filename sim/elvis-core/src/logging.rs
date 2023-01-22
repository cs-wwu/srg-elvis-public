//! Contains basic logging functions.
//!
//! Logging holds wrapper functions for logging events
//! Each function corresponds to a type of logging (messages, machine creation, etc..)
//! These functions are meant to be called from inside elvis core in the core protocols
//! Messages will be logged as Bytes in Hex formatting for most convenient parsing

use crate::{id::Id, protocols::ipv4::Ipv4Address, Message};
use tracing::{event, Level};

/// Send message event handler.
/// Used to log any messages sent. Captures the following data:
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn send_message_event(
    local_ip: Ipv4Address,
    remote_ip: Ipv4Address,
    local_port: u16,
    remote_port: u16,
    message: Message,
) {
    event!(
        target: "SEND_MESSAGE",
        Level::INFO,
        local_ip = format!("{:?}", local_ip.to_bytes()),
        remote_ip= format!("{:?}", remote_ip.to_bytes()),
        local_port= format!("{:x}", local_port),
        remote_port=format!("{:x}", remote_port),
        message=format!("{}", message),
    );
}

/// Receive message event handler.
/// Used to log any messages received. Captures the following data:
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn receive_message_event(
    local_ip: Ipv4Address,
    remote_ip: Ipv4Address,
    local_port: u16,
    remote_port: u16,
    message: Message,
) {
    event!(
        target: "RECV_MESSAGE",
        Level::INFO,
        local_ip = format!("{:?}", local_ip.to_bytes()),
        remote_ip= format!("{:?}", remote_ip.to_bytes()),
        local_port= format!("{:x}", local_port),
        remote_port=format!("{:x}", remote_port),
        message=format!("{}", message)
    );
}

// TODO: correlate the machine id's to IP's or protocols
/// Machine creation event handler.
/// Used to log the creation of any machines added to the sim. Will log:
/// machine_id, list of all protocol id's associated with the machine
/// This will eventually contain more info as the simulation evolves
pub fn machine_creation_event(protocol_ids: Vec<Id>) {
    event!(
        target: "MACHINE_CREATION",
        Level::INFO,
        protocol_ids = format!("{:?}", protocol_ids),
    );
}
