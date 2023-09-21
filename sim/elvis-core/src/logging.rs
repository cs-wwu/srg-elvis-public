//! Contains basic logging functions.
//!
//! Logging holds wrapper functions for logging events
//! Each function corresponds to a type of logging (messages, machine creation, etc..)
//! These functions are meant to be called from inside elvis core in the core protocols
//! Messages will be logged as Bytes in Hex formatting for most convenient parsing

use crate::{protocols::ipv4::Ipv4Address, Message};
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
        local_port= format!("{local_port:x}"),
        remote_port=format!("{remote_port:x}"),
        message=format!("{message}"),
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
        local_port= format!("{local_port:x}"),
        remote_port=format!("{remote_port:x}"),
        message=format!("{message}")
    );
}
