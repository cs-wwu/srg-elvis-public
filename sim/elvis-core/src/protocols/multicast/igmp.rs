//! An implementation of [Internet Group Managemet
//! Protocol](https://datatracker.ietf.org/doc/html/rfc1112)

use crate::protocols::ipv4::Ipv4Address;

pub struct MulticastAddress(Ipv4Address);

impl MulticastAddress{
    pub const fn new(address: [u8; 4]) -> Self {
        // Check the high-order four bits are "1110"
        if address[0] != 0b1110 {
            panic!("Invalid multicast address")
        }
        MulticastAddress(address)
    }
}