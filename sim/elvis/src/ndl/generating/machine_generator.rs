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
use itertools::Itertools;
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
            assert!(
                networks.contains_key(
                    net.options
                        .get("id")
                        .expect("No ID found in network being added to machine.")
                ),
                "Invalid Network ID found. Got {} expected {:?}",
                net.options.get("id").unwrap(),
                networks.keys().sorted().join(" , ")
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
                        panic!(
                            "Invalid Protocol found in machine. Found: {}",
                            option.1.as_str()
                        )
                    }
                }
            }
        }
        for app in &machine.interfaces.applications {
            // TODO: add test for this error
            assert!(
                app.options.contains_key("name"),
                "Machine application does not contain a name"
            );
            let app_name = app.options.get("name").unwrap().as_str();
            match app_name {
                "send_message" => {
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
                    assert!(
                        app.options.contains_key("port"),
                        "Capture application doesn't contain port."
                    );
                    assert!(
                        app.options.contains_key("ip"),
                        "Capture application doesn't contain ip."
                    );

                    // TODO: Check that this IP is valid in the IP table/Network
                    let ip = ip_string_to_ip(
                        app.options.get("ip").unwrap().to_string(),
                        "capture declaration",
                    );
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    protocols_to_be_added.push(Capture::new_shared(ip.into(), port));
                }

                _ => {
                    panic!(
                        "Invalid application in machine. Got application {}",
                        app_name
                    );
                }
            }
        }

        internet.machine(protocols_to_be_added, networks_to_be_added);
    }
}
