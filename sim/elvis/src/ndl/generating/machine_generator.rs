//! Generates machines from a given parse
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::{
    internet::NetworkHandle,
    protocol::SharedProtocol,
    protocols::ipv4::IpToNetwork,
    protocols::{ipv4::Ipv4, udp::Udp},
    Internet,
};
use std::collections::{HashMap, HashSet};

/// Machine Generator generates machines from a given [Machines] struct and places them in the [Internet]
pub fn machine_generator(
    m: Machines,
    internet: &mut Internet,
    networks: HashMap<String, (NetworkHandle, IpToNetwork, HashSet<[u8; 4]>)>,
) {
    // For now options are ignored as we can't currently add names/IDs to machines
    // Focusing on Interfaces, protocols, and applications
    for machine in &m {
        let mut networks_to_be_added = Vec::new();
        let mut protocols_to_be_added = Vec::new();
        let mut ip_table: IpToNetwork = IpToNetwork::new();
        for net in &machine.interfaces.networks {
            // TODO: test and change errors
            assert!(
                networks.contains_key(net.options.get("id").expect("Invalid ID found")),
                "Invalid ID found assert"
            );
            let network_adding = networks.get(net.options.get("id").unwrap()).unwrap();
            networks_to_be_added.push(network_adding.0);

            // add the IP table found into our existing IP table
            ip_table.extend(network_adding.1.clone());
        }

        for protocol in &machine.interfaces.protocols {
            for option in &protocol.options {
                match option.1.as_str() {
                    "UDP" => protocols_to_be_added.push(Udp::new_shared() as SharedProtocol),
                    "IPv4" => protocols_to_be_added.push(Ipv4::new_shared(ip_table.clone())),
                    _ => {
                        // TODO: when machine ID/name get found, add to the error
                        panic!("Invalid Protocol found in machine")
                    }
                }
            }
        }
        for app in &machine.interfaces.applications {
            // TODO: assert to check for name
            let app_name = app.options.get("name").unwrap().as_str();
            match app_name {
                "send_message" => {
                    // TODO: write the error messages for these asserts
                    assert!(app.options.contains_key("port"));
                    assert!(app.options.contains_key("to"));
                    assert!(app.options.contains_key("message"));

                    // TODO: edit this error message?
                    let to = ip_string_to_ip(
                        app.options.get("to").unwrap().to_string(),
                        "send_message declaration",
                    );
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    let message = app.options.get("message").unwrap().to_owned();

                    //TODO: ask Tim about this message Box stuff
                    protocols_to_be_added.push(SendMessage::new_shared(
                        Box::leak(message.into_boxed_str()),
                        to.into(),
                        port,
                    ));
                }

                "capture" => {
                    assert!(app.options.contains_key("port"));
                    assert!(app.options.contains_key("ip"));
                    // TODO: Check that this IP is valid in the IP table/Network
                    let ip = ip_string_to_ip(
                        app.options.get("ip").unwrap().to_string(),
                        "capture declaration",
                    );
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    protocols_to_be_added.push(Capture::new_shared(ip.into(), port));
                }

                _ => {}
            }
        }

        internet.machine(protocols_to_be_added, networks_to_be_added);
    }
}
