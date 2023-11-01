//! Various methods that are used to assist generating the simulation.

use std::collections::HashMap;

use elvis_core::protocols::{arp::subnetting::Ipv4Net, ipv4::Ipv4Address};

use crate::ip_generator::IpGenerator;

/// Converts from either a hex value or decimal value in a String and turns it into a u16 as a port.
///
/// Ex: Converts 0xbeef into u16
pub fn string_to_port(p: String) -> u16 {
    if let Some(stripped) = p.strip_prefix("0x") {
        u16::from_str_radix(stripped, 16)
            .unwrap_or_else(|_| panic!("Port declaration error. Found port: {p}"))
    } else {
        p.parse::<u16>()
            .unwrap_or_else(|_| panic!("Port declaration error. Found port: {p}"))
    }
}

/// Converts from an IP String into a [u8; 4] array.
/// Takes in a string for the IP as well as the network that IP is on for error handling.
///
/// Ex: Turns "192.168.1.121" into [192, 168, 1, 121]
pub fn ip_string_to_ip(s: String, net_id: &str) -> [u8; 4] {
    let mut new_ip: Vec<u8> = Vec::new();
    for val in s.split('.').collect::<Vec<&str>>().iter() {
        new_ip.push(val.parse::<u8>().unwrap_or_else(|_| {
            panic!("Invalid IP octet expected u8. In Network {net_id}, found: {val}")
        }));
    }

    assert!(
        new_ip.len() == 4,
        "Network {}: Invalid IP octect count, expected 4 octets found {} octets",
        net_id,
        new_ip.len()
    );

    [new_ip[0], new_ip[1], new_ip[2], new_ip[3]]
}

/// Determines if a given string is an IP or a machine name
/// Returns true if it is an IP and false if it could be a name
pub fn ip_or_name(s: String) -> bool {
    let sections: Vec<&str> = s.split('.').collect();
    if sections.len() != 4 {
        return false;
    } else {
        for section in sections {
            let octet = section.parse::<u8>();
            // errors on non-numbers and numbers > 255
            if octet.is_err() {
                return false;
            }
        }
    }

    true
}

/// Checks if a requested ip is still available
/// If available it is blocked for future use and returns the IP
/// If unavailable None is returned and the value is currently in use by another machine
pub fn ip_available(
    target_ip: Ipv4Address,
    ip_gen: &mut HashMap<String, IpGenerator>,
    cur_net_ids: &[String],
) -> Result<Ipv4Address, String> {
    let local_ip: Ipv4Net = Ipv4Net::new_short([127, 0, 0, 0], 8);

    //Find if the requested local_ip is still available for use.
    if !local_ip.contains(target_ip)
        && !cur_net_ids.iter().any(|id| {
            !ip_gen
                .get(id)
                .expect("Invalid network ID")
                .is_available(Ipv4Net::new_short(target_ip, 32))
        })
    {
        return Err("IP not available".to_string());
    }
    //If local ip was found then block it in all other ip generators
    for gen in ip_gen.values_mut() {
        gen.block_subnet(Ipv4Net::new_short(target_ip, 32));
    }
    Ok(target_ip)
}

/// Generates the information needed to create a router table entry from the [RouterEntry] subtype
/// Must contain dest and pci_slot but next_hop is optional
pub fn generate_router_entry(
    entry: HashMap<String, String>,
) -> (Ipv4Net, Option<Ipv4Address>, u32) {
    let dest_string = entry
        .get("dest")
        .unwrap_or_else(|| panic!("Router entry doesn't have a dest parameter"))
        .to_string();

    let pci_slot_string = entry
        .get("pci_slot")
        .unwrap_or_else(|| panic!("Router entry doesn't have a pci_slot parameter"))
        .to_string();

    let next_hop = entry
        .get("next_hop")
        .map(|ip_str| Ipv4Address::new(ip_string_to_ip(ip_str.to_string(), "next hop")));

    let dest = Ipv4Net::new_short(ip_string_to_ip(dest_string, "dest"), 32);
    let pci_slot: u32 = pci_slot_string.parse().unwrap();
    (dest, next_hop, pci_slot)
}
