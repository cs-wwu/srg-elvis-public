//! Generates applications from parsing data for machines
//! Future applications can go here for easy import to the machine generator
use std::collections::HashMap;

use crate::applications::{Forward, PingPong, DhcpServer};
use crate::ip_generator::{IpGenerator, IpRange};
use crate::ndl::generating::generator_utils::{ip_available, ip_or_name};
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::protocols::arp::subnetting::{Ipv4Mask, SubnetInfo};
use elvis_core::protocols::dhcp_client::DhcpClient;
use elvis_core::protocols::ipv4::{Ipv4Address, Recipient};
use elvis_core::protocols::Arp;
use elvis_core::protocols::{Endpoint, Endpoints};
use elvis_core::subnetting::Ipv4Net;
use elvis_core::{IpTable, Message};
/// Builds the [SendMessage] application for a machine
pub fn send_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
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

    let target_ip = app
        .options
        .get("ip")
        .map(|ip_str| ip_string_to_ip(ip_str.to_string(), "ip for send_message").into())
        .unwrap_or_else(|| Ipv4Address::new([127, 0, 0, 1])); //Default to local ip if none is provided

    // Check if IP is available
    let ip = ip_available(target_ip, ip_gen, cur_net_ids).expect("send_message IP unavailable");

    ip_table.add_direct(ip, Recipient::new(0, None));

    let to = app.options.get("to").unwrap().to_string();

    println!("send message: {:?} to : {:?}", ip, to);
    let port = string_to_port(app.options.get("port").unwrap().to_string());
    let message = app.options.get("message").unwrap().to_owned();
    let message = Message::new(message);
    let messages = vec![message];
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
        )
        .local_ip(target_ip)
    } else {
        SendMessage::new(
            messages,
            Endpoint {
                address: *name_to_ip.get(&to).unwrap_or_else(|| {
                    panic!("Invalid name for 'to' in send_message, found: {to}")
                }),
                port,
            },
        )
        .local_ip(target_ip)
    }
}

/// Builds the [Capture] application for a machine
pub fn capture_builder(
    app: &Application,
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
) -> Capture {
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

    // Check if IP is available
    let ip = ip_available(ip.into(), ip_gen, cur_net_ids).expect("capture IP unavailable");

    ip_table.add_direct(ip, Recipient::new(0, None));

    Capture::new(Endpoint { address: ip, port }, message_count)
}

/// Builds the [Forward] application for a machine
/// Forward on 2/6/23 by default will handle recieving and sending many messages without count
pub fn forward_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
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

    // Check if IP is available
    let ip = ip_available(ip.into(), ip_gen, cur_net_ids).expect("forward IP unavailable");

    ip_table.add_direct(ip, Recipient::new(0, None));

    if ip_or_name(to.clone()) {
        let to = ip_string_to_ip(to, "Forward declaration");
        Forward::new(Endpoints {
            local: Endpoint {
                address: ip,
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
                address: ip,
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
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
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
    // Check if IP is available
    let ip = ip_available(ip.into(), ip_gen, cur_net_ids).expect("ping_pong IP unavailable");

    ip_table.add_direct(ip, Recipient::new(0, None));

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
        //case : to is an IP
        let to = ip_string_to_ip(to, "Forward declaration");
        // case where ip to mac doesn't have a mac
        let endpoints = Endpoints {
            local: Endpoint {
                address: ip,
                port: local_port,
            },
            remote: Endpoint {
                address: to.into(),
                port: remote_port,
            },
        };
        PingPong::new(starter, endpoints)
    } else {
        // case : to is a machine name
        let endpoints = Endpoints {
            local: Endpoint {
                address: ip,
                port: local_port,
            },
            remote: Endpoint {
                address: *name_to_ip
                    .get(&to)
                    .unwrap_or_else(|| panic!("Invalid name for 'to' in PingPong, found: {to}")),
                port: remote_port,
            },
        };
        PingPong::new(starter, endpoints)
    }
}

/// Builds an [Arp] protocol for the machine
/// If a local subnet is specified a preconfigured subnet is configured
/// Otherwise a default arp is provided
pub fn arp_builder(
    name_to_ip: &HashMap<String, Ipv4Address>,
    options: &HashMap<String, String>,
) -> Arp {
    if options.contains_key("local") {
        assert!(
            options.contains_key("default"),
            "Arp protocol doesn't contain default."
        );
        let default = options.get("default").unwrap().to_string();
        let default_gateway = match ip_or_name(default.clone()) {
            true => Ipv4Address::new(ip_string_to_ip(default, "default arp id")),
            false => match name_to_ip.contains_key(&default) {
                true => *name_to_ip.get(&default).unwrap(),
                false => panic!("Invalid name for default arp gateway"),
            },
        };
        Arp::new().preconfig_subnet(
            Ipv4Address::new(ip_string_to_ip(
                options.get("local").unwrap().to_string(),
                "local arp ip",
            )),
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(32),
                default_gateway,
            },
        )
    } else {
        Arp::new()
    }
}

pub fn dhcp_server_builder(
    app: &Application,
    _name_to_ip: &HashMap<String, Ipv4Address>,
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
) -> DhcpServer {
    assert!(
        app.options.contains_key("ip"),
        "No server ip is provided for the dhcp_server application"
    );
    assert!(
        app.options.contains_key("ip_range"),
        "No ip_range is provided for the dhcp_server application"
    );
    let ip = ip_string_to_ip(
        app.options.get("ip").unwrap().to_string(),
        "dhcp_server declaration",
    );
    // Check if IP is available
    let ip = ip_available(ip.into(), ip_gen, cur_net_ids).expect("dhcp_server IP unavailable");
    ip_table.add_direct(ip, Recipient::new(0, None));
    
    let ip_range = app.options.get("ip_range").unwrap().to_string();
    let range: Vec<&str> = ip_range.split('-').collect();
    assert_eq!(range.len(), 2);
    let start = ip_string_to_ip(range[0].to_string(), &cur_net_ids[0]);
    let ceiling = range[1].parse::<u8>().unwrap_or_else(|_| {
        panic!("Dhcp server {}: Invalid ending IP range number. Expected <u8> found: {}", &cur_net_ids[0], range[1])
    });

    assert!(
        ceiling >= start[3],
        "Dhcp server {}: Invalid Cidr format, end IP value ({}) greater than start IP value ({})",
        &cur_net_ids[0], ceiling, start[3]
    );

    let end = [start[0], start[1], start[2], ceiling];
    //TODO create ip range from param
    let ip_range = IpRange::new(start.into(), end.into());

    
    DhcpServer::new(ip, ip_range)            
}

pub fn dhcp_client_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_table: &mut IpTable<Recipient>,
) -> DhcpClient {
    assert!(
        app.options.contains_key("server_ip"),
        "No server ip is provided for the dhcp_client application"
    );
    let server_ip = app.options.get("server_ip").unwrap().to_string();
    
    ip_table.add_cidr("0.0.0.0/0", Recipient::new(0, None));
    if ip_or_name(server_ip.clone()) {
        //Case: A decimal format ip is provided
        DhcpClient::new(ip_string_to_ip(server_ip, "dhcp_client declaration").into())
    } else {
        //Case: A name format ip is provided
        let server_address = *name_to_ip
            .get(&server_ip)
            .unwrap_or_else(|| panic!("Invalid name for 'server_address' in dhcp_client, found: {server_ip}"));
        DhcpClient::new(server_address)            
    }
}
