//! Generates applications from parsing data for machines
//! Future applications can go here for easy import to the machine generator
use std::collections::HashMap;
use std::sync::Arc;

use crate::applications::{Forward, PingPong};
use crate::ndl::generating::generator_utils::ip_or_name;
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::network::Mac;
use elvis_core::protocols::ipv4::{IpToTapSlot, Ipv4Address};
use elvis_core::protocols::UserProcess;

/// Builds the [SendMessage] application for a machine
pub fn send_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    name_to_mac: &HashMap<String, Mac>,
    ip_to_mac: &HashMap<Ipv4Address, Mac>,
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

    let to = app.options.get("to").unwrap().to_string();
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message = app.options.get("message").unwrap().to_owned();

    // Determines whether or not we are using an IP or a name to send this message
    if ip_or_name(to.clone()) {
        let to = ip_string_to_ip(to, "Send_Message declaration");

        // case where ip to mac doesn't have a mac
        if !ip_to_mac.contains_key(&to.into()) {
            SendMessage::new_shared(message, to.into(), port, None, 1)
        }
        // case where ip to mac does have a mac
        else {
            return SendMessage::new_shared(
                message,
                to.into(),
                port,
                Some(*ip_to_mac.get(&to.into()).unwrap()),
                1,
            );
        }
    } else {
        return SendMessage::new_shared(
            message,
            *name_to_ip
                .get(&to)
                .unwrap_or_else(|| panic!("Invalid name for 'to' in send_message, found: {to}")),
            port,
            Some(
                *name_to_mac.get(&to).unwrap_or_else(|| {
                    panic!("Invalid name for 'to' in send_message, found: {to}")
                }),
            ),
            1,
        );
    }
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
    Capture::new_shared(ip.into(), port, message_count)
}

/// Builds the [Forward] application for a machine
/// Forward on 2/6/23 by default will handle recieving and sending many messages without count
pub fn forward_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    name_to_mac: &HashMap<String, u64>,
    ip_to_mac: &HashMap<Ipv4Address, Mac>,
) -> Arc<UserProcess<Forward>> {
    assert!(
        app.options.contains_key("local_port"),
        "Forward application doesn't contain local port."
    );
    assert!(
        app.options.contains_key("remote_port"),
        "Forward application doesn't contain remote port."
    );
    assert!(
        app.options.contains_key("to"),
        "Forward application doesn't contain to address."
    );
    assert!(
        app.options.contains_key("ip"),
        "Forward application doesn't contain ip address to capture on."
    );

    let to = app.options.get("to").unwrap().to_string();
    let ip = ip_string_to_ip(
        app.options.get("ip").unwrap().to_string(),
        "Forward declaration",
    );
    let local_port = string_to_port(app.options.get("local_port").unwrap().to_string());
    let remote_port = string_to_port(app.options.get("remote_port").unwrap().to_string());
    if ip_or_name(to.clone()) {
        let to = ip_string_to_ip(to, "Forward declaration");

        // case where ip to mac doesn't have a mac
        if !ip_to_mac.contains_key(&to.into()) {
            Forward::new_shared(ip.into(), to.into(), local_port, remote_port, None)
        }
        // case where ip to mac does have a mac
        else {
            return Forward::new_shared(
                ip.into(),
                to.into(),
                local_port,
                remote_port,
                Some(*ip_to_mac.get(&to.into()).unwrap()),
            );
        }
    } else {
        Forward::new_shared(
            ip.into(),
            *name_to_ip
                .get(&to)
                .unwrap_or_else(|| panic!("Invalid name for 'to' in forward, found: {to}")),
            local_port,
            remote_port,
            Some(
                *name_to_mac
                    .get(&to)
                    .unwrap_or_else(|| panic!("Invalid name for 'to' in forward, found: {to}")),
            ),
        )
    }
}

// PingPong::new_shared(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef),

/// Builds the [PingPong] application for a machine
/// TODO: Currently shows errors in the log. I believe this is from an underlying PingPong issue.
pub fn ping_pong_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_to_mac: &HashMap<Ipv4Address, Mac>,
) -> Arc<UserProcess<PingPong>> {
    assert!(
        app.options.contains_key("local_port"),
        "Forward application doesn't contain local port."
    );
    assert!(
        app.options.contains_key("remote_port"),
        "PingPong application doesn't contain remote port."
    );
    assert!(
        app.options.contains_key("to"),
        "PingPong application doesn't contain to address."
    );
    assert!(
        app.options.contains_key("ip"),
        "PingPong application doesn't contain ip address to capture on."
    );
    assert!(
        app.options.contains_key("starter"),
        "PingPong application doesn't contain starter value."
    );
    let ip = ip_string_to_ip(
        app.options.get("ip").unwrap().to_string(),
        "PingPong declaration",
    );
    let to = app.options.get("to").unwrap().to_string();
    let local_port = string_to_port(app.options.get("local_port").unwrap().to_string());
    let remote_port = string_to_port(app.options.get("remote_port").unwrap().to_string());
    let starter: bool = match app.options.get("starter").unwrap().to_lowercase().as_str() {
        "true" => true,
        "t" => true,
        "false" => false,
        "f" => false,
        _ => false,
    };
    if ip_or_name(to.clone()) {
        let to = ip_string_to_ip(to, "Forward declaration");
        //PingPong currently does not support mac, thus this check is unnecesary currently
        // case where ip to mac doesn't have a mac
        if !ip_to_mac.contains_key(&to.into()) {
            PingPong::new_shared(starter, ip.into(), to.into(), local_port, remote_port)
        }
        // case where ip to mac does have a mac
        else {
            PingPong::new_shared(starter, ip.into(), to.into(), local_port, remote_port)
        }
    } else {
        PingPong::new_shared(
            starter,
            ip.into(),
            *name_to_ip
                .get(&to)
                .unwrap_or_else(|| panic!("Invalid name for 'to' in PingPong, found: {to}")),
            local_port,
            remote_port,
        )
    }
}
