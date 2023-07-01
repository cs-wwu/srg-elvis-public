use std::collections::BTreeMap;

use crate::protocols::arp::subnetting::*;
use crate::protocols::ipv4::Ipv4Address;

type Entry = (Ipv4Address, Ipv4Mask);

pub struct IpTable<T: Copy> {
    table: BTreeMap<Entry, T>,
    // mapping to keep track of number of num of unique subnets associated with
    // each mask
    masks: BTreeMap<Ipv4Mask, u32>,
}

impl<T: Copy> IpTable<T> {
    pub fn new() -> Self {
        IpTable {
            table: Default::default(),
            masks: Default::default(),
        }
    }

    /// Specifies the default recipient to send packets to if no other subnet
    /// is found in the table.
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn default_gateway(recipient: T) -> Self {
        let mut table = IpTable::new();
        table.add(cidr_to_ip("0.0.0.0/0").unwrap(), recipient);
        table
    }

    /// Gets value associated with provided ipv4 address by starting at the most
    /// specific mask in the table (32 if there are mappings to that mask) and 
    /// returns the destination associated with that mask. If no subnet is found,
    /// the recipient linked to the default gateway is returned. If no default gateway is 
    /// specified an error is returned.
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn get_recipient(&mut self, address: Ipv4Address) -> Result<T, ()> {
        for entry in self.masks.keys().rev() {
            let masked_address = get_network_id(address, *entry);
            if let Some(recipient) = self.table.get(&(masked_address, *entry)) {
                return Ok(*recipient);
            }
        }
        Err(())
    }

    /// Removes subnet associated with given key from the table
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn remove(&mut self, key: Entry) {
        let masked_key = (get_network_id(key.0, key.1), key.1);

        match self.table.remove(&masked_key) {
            None => return,
            Some(_) => {},
        }
        
        match self.masks.get(&key.1) {
            Some(&val) => {
                if val == 1 {
                    self.masks.remove(&key.1);
                } else {
                    self.masks.insert(key.1, val - 1);
                }
            }
            None => (),
        }
    }

    /// Removes address associated with given ip address using
    /// 32 bit mask length as second part of the key. 
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn remove_direct(&mut self, address: Ipv4Address) {
        self.remove((address, Ipv4Mask::from_bitcount(32)));
    }

    /// Removes address associated with given ip address using
    /// cidr notation. If notation is invalid the table
    /// is left unchanged
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn remove_cidr(&mut self, cidr: &str) {
        match cidr_to_ip(cidr) {
            Ok(key) => {
                self.remove(key);
            }
            Err(_) => (),
        }
    }

    /// Maps subnet associated with (Ipv4Address, Ipv4Mask) pair
    /// to provided value. 
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn add(&mut self, key: Entry, value: T) {
        let masked_key = (get_network_id(key.0, key.1), key.1);

        if let Some(_) = self.table.insert(masked_key, value) {
            return;
        }

        let total = match self.masks.get(&key.1) {
            Some(val) => *val,
            None => 0,
        };

        self.masks.insert(key.1, total + 1);
    }

    /// Maps ipv4 address associated with address/32
    /// to provided value. 
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn add_direct(&mut self, address: Ipv4Address, value: T) {
        self.add((address, Ipv4Mask::from_bitcount(32)), value);
    }

    /// Maps subnet associated with given cidr notation to
    /// to provided value. 
    /// 
    /// # Examples
    /// ```
    /// 
    /// ```
    pub fn add_cidr(&mut self, cidr: &str, value: T) {
        match cidr_to_ip(cidr) {
            Ok(key) => {
                self.add(key, value);
            }
            Err(_) => (),
        }
    }
}

mod test {
    use super::*;

    #[allow(dead_code)]
    fn setup() -> IpTable<u32> {
        let mut table = IpTable::new();

        table.add_cidr("1.0.0.0/8", 0);
        table.add_cidr("1.1.0.0/16", 1);
        table.add_cidr("1.1.1.0/24", 2);
        table.add_cidr("1.1.2.0/24", 3);
        table.add_cidr("1.1.3.0/24", 4);
        table.add_cidr("1.2.3.0/24", 5);
        table.add_cidr("1.1.1.2/32", 6);
        table.add_cidr("1.1.3.4/32", 7);

        table
    }

    #[allow(dead_code)]
    pub fn print_table(table: &IpTable<u32>) {
        for entry in table.table.iter() {
            println!("{:?}", entry);
        }

        println!();

        for entry in table.masks.iter() {
            println!("{:?}", entry);
        }
    }

    #[test]
    fn test_add() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1, 1, 1, 2]);
        let ip2 = Ipv4Address::new([1, 1, 0, 1]);

        assert_eq!(table.get_recipient(ip1), Ok(6));
        assert_eq!(table.get_recipient(ip2), Ok(1));

        table.add(cidr_to_ip("1.1.0.0/24").unwrap(), 20);

        assert_eq!(table.get_recipient(ip2), Ok(20));
    }

    #[test]
    fn test_remove() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1, 2, 3, 4]);

        assert_eq!(table.get_recipient(ip1), Ok(7));

        table.remove(cidr_to_ip("1.2.3.4/32").unwrap());

        assert_eq!(table.get_recipient(ip1), Ok(5));

        table.remove(cidr_to_ip("1.1.1.2/32").unwrap());

        // all 32 bit mask addresses should now be removed from the table 
        assert_eq!(table.masks.get(&Ipv4Mask::from_bitcount(32)), None);
    }

    fn test_create() {
        vec![2];
    }
}
