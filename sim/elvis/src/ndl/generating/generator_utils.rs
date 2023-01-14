//! Various methods that are used to assist generating the simulation.

/// Converts from either a hex value or decimal value in a String and turns it into a u16 as a port.
/// 
/// Ex: Converts 0xbeef into u16
pub fn string_to_port(p: String) -> u16{
    if p.starts_with("0x"){
        return u16::from_str_radix(&p[2..], 16).expect(&format!("Port declaration error. Found port: {}", p));
    }
    else {
        return p.parse::<u16>().expect(&format!("Invalid number for port. Found port: {}", p));
    }
}

/// Converts from an IP String into a [u8; 4] array.
/// Takes in a string for the IP as well as the network that IP is on for error handling.
/// 
/// Ex: Turns "192.168.1.121" into [192, 168, 1, 121]
pub fn ip_string_to_ip(s: String, net_id: &str) -> [u8; 4] {
    let temp: Vec<&str> = s.split(".").collect();
    let mut new_ip: Vec<u8> = Vec::new();
    for val in temp {
        new_ip.push(val.parse::<u8>().expect(&format!(
            "Network {}: Invalid IP octet (expecting a u8)",
            net_id
        )));
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
