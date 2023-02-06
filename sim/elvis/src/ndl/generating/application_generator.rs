//! Generates applications from parsing data for machines
//! Future applications can go here for easy import to the machine generator
use std::collections::HashMap;
use std::sync::Arc;

use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::protocols::ipv4::{IpToTapSlot, Ipv4Address};
use elvis_core::protocols::UserProcess;

/// Builds the [SendMessage] application for a machine
pub fn send_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    name_to_mac: &HashMap<String, u64>,
) -> Arc<UserProcess<SendMessage>> {
    assert!(
        app.options.contains_key("port"),
        "Send_Message application doesn't contain port."
    );
    assert!(
        app.options.contains_key("to"),
        "Send_Message application doesn't contain to address."
    );
    assert!(
        app.options.contains_key("message"),
        "Send_Message application doesn't contain message."
    );

    // TODO: might want to include both these behaviors (so they could enter IP or enter name)
    let to = app.options.get("to").unwrap().to_string();
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message = app.options.get("message").unwrap().to_owned();
    //TODO: ask Tim about this message Box stuff
    SendMessage::new_shared(
        message,
        *name_to_ip
            .get(&to)
            .unwrap_or_else(|| panic!("Invalid name for 'to' in send_message, found: {to}")),
        port,
        Some(
            *name_to_mac
                .get(&to)
                .unwrap_or_else(|| panic!("Invalid name for 'to' in send_message, found: {to}")),
        ),
        1,
    )
}

/// Builds the [Capture] application for a machine
pub fn capture_builder(app: &Application, ip_table: &IpToTapSlot) -> Arc<UserProcess<Capture>> {
    assert!(
        app.options.contains_key("port"),
        "Capture application doesn't contain port."
    );
    assert!(
        app.options.contains_key("ip"),
        "Capture application doesn't contain ip."
    );
    assert!(
        app.options.contains_key("message_count"),
        "Capture application doesn't contain message_count."
    );
    let ip = ip_string_to_ip(
        app.options.get("ip").unwrap().to_string(),
        "capture declaration",
    );
    assert!(
        ip_table.contains_key(&ip.into()),
        "Invalid IP found in capture application. IP does not exist in ip table. Found: {ip:?}"
    );
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message_count = app
        .options
        .get("message_count")
        .unwrap()
        .parse::<u32>()
        .expect("Invalid u32 found in Capture for message count");
    // TODO: Figure out how to get actual number to recieve in
    // TODO: Add message expected count
    // maybe default to 1?
    Capture::new_shared(ip.into(), port, message_count)
}
