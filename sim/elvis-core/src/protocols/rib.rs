/// TODO!
/// an RIB or routing information base is a Table
/// used to store information about the topology of a network.
/// This table allows a router to know where to send recieved messages
/// by first applying a mask to the recieved address and putting the masked
/// address through a hash. Masking allows for multiple ip addresses to be mapped to one destination
struct Rib {
    // what should the result map to?
    table: HashMap<Ipv4Address, Entry>,

    // the masks to apply to the given ip address
    // should be ordered from smallest to largest
    masks: Vec<Mask>,
}

pub struct Mask(u32);

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
}