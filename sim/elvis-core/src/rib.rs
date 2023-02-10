
use std::{collections::{HashMap, BTreeMap, BTreeSet}, fmt::{Formatter, self}};
use crate::protocols::ipv4::{Ipv4Address, IpToTapSlot};

// Mask needs to be ordered so mask of all ones is smallest value
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub struct SubnetMask(u32);

impl SubnetMask {
    pub const DEFAULT_GATEWAY: Self = SubnetMask(0);

    /// returns a mask of size ones
    /// input should be a number from 0 to 32
    /// need to remove branch to make faster but this will be for another time
    pub fn from_bitcount(size: u32) -> SubnetMask {
        let size = size.clamp(0,32);
        if size == 0 {
            return SubnetMask(0)
        } else if size == 32 {
            return SubnetMask(0xffffffff)
        }
        SubnetMask(((1 << size) - 1) << (32 - size))
        // let size = size.clamp(0, 32);
        // SubnetMask((0xffffffff as u32).wrapping_shl(size))
    }

    // change to try from to ensure valid subnetmask
    pub fn from_u32(value: u32) -> SubnetMask {
        SubnetMask(value)
    }

    pub fn mask(self, addr: Ipv4Address) -> Ipv4Address {
        Ipv4Address::from(addr.to_u32() & self.to_u32())
    }

    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

/// TODO!
/// a RIB or routing information base is a Table
/// used to store information about the topology of a network.
/// This table allows a router to know where to send recieved messages
/// by first applying a mask to the recieved address and putting the masked
/// address through a hash. Masking allows for multiple ip addresses to be mapped to one destination

#[derive(Default, Debug)]
pub struct Rib {
    // maps an Ipv4 address to a router table entry
    table: IpToTapSlot,

    // the masks to apply to the given ip address
    // should be ordered from 'largest' (i.e all ones) to 'smallest' (i.e all zeros)
    // so that we prefer specific ip routes over fuzzy ones
    // a mask of all 0s represents the default gateway
    masks: BTreeSet<Key>,
}

// do we need this?
// impl Compare for SubnetMask {
//     fn compare(&self, l: &L, r: &R) {

//     }
// }

/// Entry contains the information about the next destination of the message
/// we will keep the reciever as a string for now
pub struct Entry {
    reciever: String
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Key {
    mask: SubnetMask,
    addr: Ipv4Address
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("reciever", &self.reciever)
            .finish()
    }
}

impl Default for Entry {
    fn default() -> Self {
        Entry {reciever: String::new() }
    }
}

impl Rib {
    /// TODO!
    /// obtains the location of the next tap to forward to
    /// this is done by iterating through the masks from smallest (all ones) to largest (all zeros)
    /// and applying the mask to the ipv4address
    /// the masked entry is then put through a hashmap and is returned if a match is found
    /// if no match is found then discard the message and return none
    pub fn new() -> Self {
        Rib {table: IpToTapSlot::new(), masks: BTreeSet::new()}
    }

    pub fn get(&mut self, address: Ipv4Address) -> Option<Entry> {
        // self.masks.into_iter().rev().
        todo!()
    }

    // maps given ip address to given entry
    pub fn put(&mut self, address: Ipv4Address, mask: SubnetMask, entry: Entry) {

    }

    pub fn print(self) {
        // for mask in self.masks {
        //     println!("{} {}", Ipv4Address::from(mask.to_u32()).to_string());
        // }
    }

    // initialize routing table from an input string for static routing
    pub fn from(input: &String) -> Rib {
        todo!()
    }

}

// Tests go here
#[cfg(test)]
mod tests {
    use crate::Message;

    use super::*;

    // mask tests
    #[test]
    pub fn test_from_bitcount() {
        // let ip = Ipv4Address::new([192,168,1,1]);
        let raw_ips = vec![[192,168,1,1], [192,168,1,0], [192,168,0,0], [192,0,0,0], [0,0,0,0]];

        let ip = Ipv4Address::new([192,168,1,1]);

        let ips: Vec<Ipv4Address> = raw_ips
            .into_iter()
            .map(|e| Ipv4Address::new(e))
            .collect();

        assert_eq!(ips[0], SubnetMask::from_bitcount(32).mask(ip));
        assert_eq!(ips[1], SubnetMask::from_bitcount(24).mask(ip));
        assert_eq!(ips[2], SubnetMask::from_bitcount(16).mask(ip));
        assert_eq!(ips[3], SubnetMask::from_bitcount( 8).mask(ip));
        assert_eq!(ips[4], SubnetMask::from_bitcount( 0).mask(ip));
    }

    // test rib
    #[test]
    pub fn test_rib_put() {
        todo!();
    }
}