//! Various methods that are used to assist generating the simulation.

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

    assert_eq!(
        new_ip.len(),
        4,
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
