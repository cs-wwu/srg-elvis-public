//! Generates applications from parsing data for machines
//! Future applications can go here for easy import to the machine generator
use std::collections::HashMap;

use crate::applications::{Forward, PingPong};
use crate::ip_generator::IpGenerator;
use crate::ndl::generating::generator_utils::{ip_or_name, ip_available};
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::protocols::ipv4::{Ipv4Address, Recipient};
use elvis_core::protocols::{Endpoint, Endpoints};
use elvis_core::{IpTable, Message};
/// Builds the [SendMessage] application for a machine
pub fn send_message_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>,
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
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

    let target_ip = app.options.get("ip")
        .map(|ip_str| ip_string_to_ip(ip_str.to_string(), "ip for send_message").into())
        .unwrap_or_else(|| Ipv4Address::new([127, 0, 0, 1])); //Default to local ip if none is provided

    //Check if ip is available
    match ip_available(target_ip, ip_gen) {
        Ok(ip) => {
            ip_table.add_direct(ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("Send_Message error: {}", err);
        }
    }
    
    

    let to = app.options.get("to").unwrap().to_string();
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
    ip_gen: &mut HashMap<String, IpGenerator>,) -> Capture {

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

    //Check if ip is available
    match ip_available(ip.into(), ip_gen) {
        Ok(ip) => {
            ip_table.add_direct(ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("Capture error: {}", err);
        }
    }

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
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,
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

    //Check if ip is available
    match ip_available(ip.into(), ip_gen) {
        Ok(ip) => {
            ip_table.add_direct(ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("Forward error: {}", err);
        }
    }

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
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,) -> PingPong {

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
    //Check if ip is available
    match ip_available(ip.into(), ip_gen) {
        Ok(ip) => {
            ip_table.add_direct(ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("PingPong error: {}", err);
        }
    }

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
                address: ip.into(),
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
        PingPong::new(starter, endpoints)
    }
}


// builds a rip router 
pub fn rip_router_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>, 
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,) {

    //checking we have an ip address parameter
    assert!(
        app.options.contains_key("ip"),
        "rip_router does not have an ip adddress."
    );
    //router ips

    //TODO support multiple local ips, figure out a good way for ndl input
        // we could add a count for number of ip's
        // then the names are ip1 ip2 ect and we can make the strings to check for them within a loop
        //maybe something along the lines of: 
        // the ndl line would be [application='rip_router' count='2' ip1 = 'insert ip here' ip2 = insert ip here']
    
    /*
    //getting the number of ips
    assert!(
        app.options.contains_key("count"),
        "rip_router does not have an ip adddress."
    );
    let count_string = entry.get("count").unwrap().to_string();
    let count : u32 = count_string.parse().unwrap();
    if n <1{
        panic!("Invalid count in rip router: {}", err);
    }
    let base_string = "ip";
    //getting each ip
    for n in 1..=count {
        let mut new_ip = base_string.clone();
        new_ip.push(n);
        assert!(
            app.options.contains_key(new_ip),
            "rip_router does not have an ip adddress."
        );
        let multiple_ip_string = entry.get("count").unwrap().to_string();
        let multiple_ip = name_or_string_ip_to_ip(multiple_ip_string, name_to_ip);;
        match ip_available(multiple_ip.into(), ip_gen) {
            Ok(multiple_ip) => {
                ip_table.add_direct(multiple_ip, Recipient::new(0, None));
            }
            Err(err) => {
                panic!("Rip router builder error: {}", err);
            }
        }
        // we could also save them into another data structure if desired 

    }
    
    */


    let ip_string = app.options.get("ip").unwrap().to_string();
    let router_ip = name_or_string_ip_to_ip( ip_string, name_to_ip);
    //TODO check local ips with ip_generator
    match ip_available(router_ip.into(), ip_gen) {
        Ok(router_ip) => {
            ip_table.add_direct(router_ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("Rip router builder error: {}", err);
        }
    }
    //TODO create a ip table from the local ips

    //router table
    // let finished_table: IpTable<(Ipv4Address, u32)> = match &app.router_table {
    //     Some(table) => {
    //         let mut router_table: IpTable<(Ipv4Address, PciSlot)> =
    //             IpTable::<(Ipv4Address, PciSlot)>::new();
    //         for entry in table.iter() {
    //             //look at add direct method 
    //             assert!(
    //                 entry.contains_key("dest"),
    //                 "Router entry doesnt have a dest parameter"
    //             );
    //             assert!(
    //                 entry.contains_key("pci_slot"),
    //                 "Router entry doesnt have a pci_slot parameter"
    //             );

    //             //code is mostly copied from boris-ellie-ndl-router 
    //             let dest_string = entry.get("dest").unwrap().to_string();
    //             let pci_slot_string = entry.get("pci_slot").unwrap().to_string();

    //             //TODO destination should support subnets
    //             // get_ip_and_mask might work here, not sure though
    //             // do we need to match the destiations with the ip gen???
    //             let pre_dest = get_ip_and_mask(dest_string, name_to_ip);
    //             let dest = Ipv4Net::new(
    //                 pre_dest.0,
    //                 Ipv4Mask::from_bitcount(pre_dest.1),
    //             );
    //             let pci_slot = pci_slot_string.parse().unwrap();
                
    //             //TODO create router table
    //             router_table.add(dest, pci_slot);
                
    //         }
    //         router_table
    //     }
    //     None => {
    //         panic!("Issue building arp router table, possibly none passed")
    //     }
    // };
    //TODO create an arp router with ip_table and router table
    //TODO create rip router with the ip_table
    
    
}


pub fn arp_router_builder(
    app: &Application,
    name_to_ip: &HashMap<String, Ipv4Address>, 
    ip_table: &mut IpTable<Recipient>,
    ip_gen: &mut HashMap<String, IpGenerator>,) -> ArpRouter{

    //checking we have an ip address parameter
    assert!(
        app.options.contains_key("ip"),
        "rip_router does not have an ip adddress."
    );

    let ip_string = app.options.get("ip").unwrap().to_string();
    let router_ip = name_or_string_ip_to_ip( ip_string, name_to_ip);
    //TODO check local ips with ip_generator
    match ip_available(router_ip.into(), ip_gen) {
        Ok(router_ip) => {
            ip_table.add_direct(router_ip, Recipient::new(0, None));
        }
        Err(err) => {
            panic!("Rip router builder error: {}", err);
        }
    }

    //router table
    let finished_table: IpTable<(Ipv4Address, u32)> = match &app.router_table {
        Some(table) => {
            let mut router_table: IpTable<(Ipv4Address, PciSlot)> =
                IpTable::<(Ipv4Address, PciSlot)>::new();
            for entry in table.iter() {
                assert!(
                    entry.contains_key("dest"),
                    "Router entry doesnt have a dest parameter"
                );
                assert!(
                    entry.contains_key("pci_slot"),
                    "Router entry doesnt have a pci_slot parameter"
                );
                assert!(
                    entry.contains_key("next_hop"),
                    "Router entry doesnt have a next_hop parameter"
                );
                let dest_string = entry.get("dest").unwrap().to_string();
                let pci_slot_string = entry.get("pci_slot").unwrap().to_string();
                let next_hop_string = entry.get("next_hop").unwrap().to_string();

                // get_ip_and_mask might work here, not sure though
                // do we need to match the destiations with the ip gen???
                let pre_dest = get_ip_and_mask(dest_string, name_to_ip);
                let dest = Ipv4Net::new(
                    pre_dest.0,
                    Ipv4Mask::from_bitcount(pre_dest.1),
                );
                let pci_slot = pci_slot_string.parse().unwrap();
                let next_hop = name_or_string_ip_to_ip( next_hop_string, name_to_ip);
                
                //TODO create router table
                router_table.add(dest, (next_hop, pci_slot));
                
            }
            router_table
        }
        None => {
            panic!("Issue building arp router table, possibly none passed")
        }
    };
    ArpRouter::new(finished_table, router_ip)
    
    
}




//takes in a string and checks if its a name, or an ip address
// if its a name it returns the coresponding ip address in ipv4 formatt
// if its just a regular ip address it returns it in the correct formatt
pub fn name_or_string_ip_to_ip(
    ip_string : String,
    name_to_ip: &HashMap<String, Ipv4Address>,
) 
-> Ipv4Address{
    let final_ip;
    if ip_or_name (ip_string.clone()){
        final_ip = Ipv4Address::new(ip_string_to_ip(ip_string, "Arp router"));
    } else {
        if name_to_ip.contains_key(&ip_string){
            final_ip = name_to_ip.get(&ip_string).unwrap().clone();
        } else {
            // here we could seperate the ip addresss from the mask
            println!("Unable to idenify name or ip {}", ip_string);
            panic!("name unknown in name_or_string_ip_to_ip")
        }
    }
    final_ip
}

//takes a string and the naming table and returns the ip adress and mask 
pub fn get_ip_and_mask(s: String, name_to_ip: &HashMap<String, Ipv4Address>,) -> (Ipv4Address, u32) {
    let seperate : Vec<&str> = s.split('/').collect();
    let address: Ipv4Address;
    let mask: u32;
    if seperate.len() == 2 {
        address = Ipv4Address::new(ip_string_to_ip(seperate[0].to_string(), "Arp router"));
        mask = seperate[1].parse().unwrap();
    } else if seperate.len() == 1 {
        if ip_or_name(seperate[0].to_string().clone()) {
            address = Ipv4Address::new(ip_string_to_ip(seperate[0].to_string(), "Arp router"));
            mask = 32;
        } else if name_to_ip.contains_key(seperate[0]){
            address = name_to_ip.get(seperate[0]).unwrap().clone();
            mask = 32;
        } else {
            panic!("Something went wrong in get_ip_and_mask");
        }
    } else {
        panic!("Something went wrong in get_ip_and_mask");
    }
    (address, mask)
}
