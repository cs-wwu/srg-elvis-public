use crate::{protocols::ipv4::Ipv4Address, network::Mac};
// use std::collections::{HashMap, BTreeMap, BTreeSet};

// Mask needs to be ordered so mask of all ones is smallest value
#[derive(Eq, PartialEq, Debug)]
pub struct SubnetMask(u32);


impl SubnetMask {
    /// returns a mask of size ones
    /// input should be a number from 0 to 32
    pub fn from_bitcount(size: u32) -> SubnetMask {
        let shift = size.clamp(0, 32);
        SubnetMask((0xffff as u32).wrapping_shl(shift)) 
    }

    pub fn from_u32(value: u32) -> SubnetMask {
        SubnetMask(value)
    }

    pub fn mask(self, addr: Ipv4Address) -> Ipv4Address {
        Ipv4Address::from(addr.to_u32() & self.to_u32())
    }

    pub fn to_u32(self) -> u32 {
        self.into()
    }
}

// as of now takes u32 as face value but we do not
// want to return masks that have zeros between 1s
impl From<SubnetMask> for u32 {
    fn from(n: SubnetMask) -> u32 {
        n.0
    }
}

/// TODO!
/// a RIB or routing information base is a Table
/// used to store information about the topology of a network.
/// This table allows a router to know where to send recieved messages
/// by first applying a mask to the recieved address and putting the masked
/// address through a hash. Masking allows for multiple ip addresses to be mapped to one destination

// pub struct Rib {
//     // maps an Ipv4 address to a router table entry
//     table: HashMap<Ipv4Address, Entry>,

//     // the masks to apply to the given ip address
//     // should be ordered from 'largest' (i.e all ones) to 'smallest' (i.e all zeros)
//     // so that we prefer specific ip routes over fuzzy ones
//     // a mask of all 0s represents the default gateway
//     masks: BTreeSet<SubnetMask>,
// }

// do we need this?
// impl Compare for SubnetMask {
//     fn compare(&self, l: &L, r: &R) {

//     }
// }

/// Entry contains the information about the next destination of the message
/// 

// struct Entry {
//     reciever: Mac
// }

// impl Rib {
//     /// TODO!
//     /// obtains the location of the next tap to forward to
//     /// this is done by iterating through the masks from smallest (all ones) to largest (all zeros)
//     /// and applying the mask to the ipv4address
//     /// the masked entry is then put through a hashmap and is returned if a match is found
//     /// if no match is found then discard the message and return none
//     pub fn get(address: Ipv4Address) -> Option<Entry> {
//         todo!();
//     }

//     // maps given ip address to given entry
//     pub fn put(address: Ipv4Address, entry: Entry) {
//         todo!();
//     }

//     // adds given mask to the RIB
//     pub fn add_mask(mask: SubnetMask) {
//         todo!();
//     }

//     // initialize routing table from an input string for static routing
//     pub fn from(input: &String) -> Rib {
//         todo!();
//     }

//     pub fn new() -> Rib {
//         todo!();
//     }
// }

// Tests go here
#[cfg(test)]
mod tests {
    use super::*;

    // methods to test should have #[test] appended
    // mask tests
    #[test]
    pub fn test_from_bitcount() {
        // let ip = Ipv4Address::new([192,168,1,1]);
        // let raw_ips = vec![[192,168,1,1], [192,168,1,0], [192,168,0,0], [192,0,0,0]];

        // let ips: Vec<Ipv4Address> = raw_ips
        //     .into_iter()
        //     .map(|e| Ipv4Address::new(e))
        //     .collect();

        // for (i, addr) in ips.iter().enumerate() {
        //     let len: u32 = i.try_into().unwrap();
        //     let test_addr = SubnetMask::from_bitcount((len - 1)*8).mask(*addr);
        //     // println!("{}", test_addr.to_string());
        //     assert_eq!(*addr, test_addr);
        // }
        
        assert_eq!(Ipv4Address::from([192,168,1,0]), 
            SubnetMask::from_bitcount(0).mask(Ipv4Address::from([192,168,1,1])));
        
    }

    // test rib
}