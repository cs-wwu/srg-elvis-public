/// TODO!
/// a RIB or routing information base is a Table
/// used to store information about the topology of a network.
/// This table allows a router to know where to send recieved messages
/// by first applying a mask to the recieved address and putting the masked
/// address through a hash. Masking allows for multiple ip addresses to be mapped to one destination
struct Rib {
    // maps an Ipv4 address to a router table entry
    table: HashMap<Ipv4Address, Entry>,

    // the masks to apply to the given ip address
    // should be ordered from 'largest' (i.e all ones) to 'smallest' (i.e all zeros)
    // so that we prefer specific ip routes over fuzzy ones
    // a mask of all 0s represents the default gateway
    masks: BTreeSet<Mask>,
}

#[derive(Eq, PartialEq)]
pub struct Mask(u32);

// do we need this?
// impl Compare for Mask {
//     fn compare(&self, l: &L, r: &R) {

//     }
// }

// this entry contains the information about the next destination of the message
// 
struct Entry {
    // possibly a weak pointer to a tap? 
    // possibly distance from the current router to the next?
    //
}

impl Rib {
    /// TODO!
    /// obtains the location of the next tap to forward to
    /// this is done by iterating through the masks from smallest to largest
    /// and applying the mask to the ipv4address
    /// the masked entry is then put through a hashmap and is returned if a match is found
    /// if no match is found then discard the message and return none
    /// if a match is found but the tap referenced no longer exists then throw an application error
    pub fn get(address: Ipv4Address) -> Option<Entry> {
        todo!();
    }

    // maps given ip address to given entry
    pub fn put(address: Ipv4Address, entry: Entry) {
        todo!();
    }

    // adds given mask to the RIB
    pub fn add_mask(mask: Mask) {
        todo!();
    }

    // initialize routing table from an input string for static routing
    pub fn from(input: &String) -> Rib {
        todo!();
    }

    pub fn new() -> Rib {
        todo!();
    }
}