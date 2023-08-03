//! Generates applications from parsing data for machines
//! Future applications can go here for easy import to the machine generator
use std::collections::HashMap;

use crate::applications::{Forward, PingPong};
use crate::ndl::generating::generator_utils::ip_or_name;
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::network::Mac;
use elvis_core::protocols::ipv4::Ipv4Address;
use elvis_core::protocols::{Endpoint, Endpoints};
use elvis_core::Message;

/// Builds the [SendMessage] application for a machine
pub fn send_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
) -> SendMessage {
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
    let local_ip = match app.options.contains_key("local_ip") {
        true => ip_string_to_ip(app.options.get("local_ip").unwrap().to_string(), "local_ip declaration").into(),
        false => Ipv4Address::new([127, 0, 0, 1]),
    };

    let to = app.options.get("to").unwrap().to_string();
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message = app.options.get("message").unwrap().to_owned();
    let message = Message::new(message);
    let messages = vec![message];
    println!("Local ip for machines: {:?}", local_ip);
    // Determines whether or not we are using an IP or a name to send this message
    if ip_or_name(to.clone()) {
        let to = ip_string_to_ip(to, "Send_Message declaration");

        // case where ip to mac doesn't have a mac
        SendMessage::new(
            messages,
            Endpoint {
                address: to.into(),
                port,
            },
        ).local_ip(local_ip)
    } else {
        SendMessage::new(
            messages,
            Endpoint {
                address: *name_to_ip.get(&to).unwrap_or_else(|| {
                    panic!("Invalid name for 'to' in send_message, found: {to}")
                }),
                port,
            },
        ).local_ip(local_ip)
    }
}

/// Builds the [Capture] application for a machine
pub fn capture_builder(app: &Application) -> Capture {
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
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message_count = app
        .options
        .get("message_count")
        .unwrap()
        .parse::<u32>()
        .expect("Invalid u32 found in Capture for message count");
    Capture::new(
        Endpoint {
            address: ip.into(),
            port,
        },
        message_count,
    )
}

/// Builds the [Forward] application for a machine
/// Forward on 2/6/23 by default will handle recieving and sending many messages without count
pub fn forward_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
) -> Forward {
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
        Forward::new(Endpoints {
            local: Endpoint {
                address: ip.into(),
                port: local_port,
            },
            remote: Endpoint {
                address: to.into(),
                port: remote_port,
            },
        })
    } else {
        Forward::new(Endpoints {
            local: Endpoint {
                address: ip.into(),
                port: local_port,
            },
            remote: Endpoint {
                address: *name_to_ip
                    .get(&to)
                    .unwrap_or_else(|| panic!("Invalid name for 'to' in forward, found: {to}")),
                port: remote_port,
            },
        })
    }
}

// PingPong::new_shared(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xbeef),

/// Builds the [PingPong] application for a machine
/// TODO: Currently shows errors in the log. I believe this is from an underlying PingPong issue.
pub fn ping_pong_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_to_mac: &HashMap<Ipv4Address, Mac>,
    name_to_mac: &HashMap<String, u64>,
) -> PingPong {
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
        // case where ip to mac doesn't have a mac
        let endpoints = Endpoints {
            local: Endpoint {
                address: ip.into(),
                port: local_port,
            },
            remote: Endpoint {
                address: to.into(),
                port: remote_port,
            },
        };
        if !ip_to_mac.contains_key(&to.into()) {
            PingPong::new(starter, endpoints)
        }
        // case where ip to mac does have a mac
        else {
            PingPong::new(starter, endpoints).remote_mac(*ip_to_mac.get(&to.into()).unwrap())
        }
    } else {
        let endpoints = Endpoints {
            local: Endpoint {
                address: ip.into(),
                port: local_port,
            },
            remote: Endpoint {
                address: *name_to_ip
                    .get(&to)
                    .unwrap_or_else(|| panic!("Invalid name for 'to' in PingPong, found: {to}")),
                port: remote_port,
            },
        };
        PingPong::new(starter, endpoints).remote_mac(
            *name_to_mac
                .get(&to)
                .unwrap_or_else(|| panic!("Invalid name for 'to' in forward, found: {to}")),
        )
    }
}
