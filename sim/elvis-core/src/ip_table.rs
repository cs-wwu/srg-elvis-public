use std::collections::BTreeMap;

use crate::protocols::ipv4::Ipv4Address;
use crate::protocols::arp::subnetting::*;

type Entry = (Ipv4Address, Ipv4Mask);

pub struct IpTable<T: Copy> {
    table: BTreeMap<Entry, T>,
    masks: BTreeMap<Ipv4Mask, u32>
}

impl<T: Copy> IpTable<T> {
    pub fn new() -> Self {
        IpTable {
            table: Default::default(),
            masks: Default::default()
        }
    }

    pub fn default_gateway(recipient: T) -> Self {
        let mut table = IpTable::new();
        table.add( cidr_to_ip("0.0.0.0/0").unwrap(), recipient);
        table
    }

    // get recipient associated with provided ipv4 address.
    // searches through list of masks and checks if the masked ip and mask pair
    // is on the table and returns the first matching recipient
    pub fn get_recipient(&mut self, address: Ipv4Address) -> Result<T, ()> {
        for entry in self.masks.keys().rev() {
            let masked_address = get_network_id(address, *entry);
            if let Some(recipient) = self.table.get(&(masked_address, *entry)) {
                return Ok(*recipient);
            }
        }
        Err(())
    }

    pub fn remove(&mut self, key: Entry) {
        let masked_key = (get_network_id(key.0, key.1), key.1);

        if !self.table.contains_key(&masked_key) {
            return;
        }

        self.table.remove(&masked_key);

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
}

mod test {
    use super::*;

    #[allow(dead_code)]
    fn setup() -> IpTable<u32> {
        let mut table = IpTable::new();

        table.add(cidr_to_ip("1.0.0.0/8").unwrap(), 0);
        table.add(cidr_to_ip("1.1.0.0/16").unwrap(), 1);
        table.add(cidr_to_ip("1.1.1.0/24").unwrap(), 2);
        table.add(cidr_to_ip("1.1.2.0/24").unwrap(), 3);
        table.add(cidr_to_ip("1.1.3.0/24").unwrap(), 4);
        table.add(cidr_to_ip("1.2.3.0/24").unwrap(), 5);
        table.add(cidr_to_ip("1.1.1.2/32").unwrap(), 6);
        table.add(cidr_to_ip("1.2.3.4/32").unwrap(), 7);

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

        let ip1 = Ipv4Address::new([1,1,1,2]);
        let ip2 = Ipv4Address::new([1,1,0,1]);

        assert_eq!(table.get_recipient(ip1), Ok(6));
        assert_eq!(table.get_recipient(ip2), Ok(1));

        table.add(cidr_to_ip("1.1.0.0/24").unwrap(), 20);

        assert_eq!(table.get_recipient(ip2), Ok(20));
    }

    #[test]
    fn test_remove() {
         let mut table = setup();

         let ip1 = Ipv4Address::new([1,2,3,4]);

         assert_eq!(table.get_recipient(ip1), Ok(7));

         table.remove(cidr_to_ip("1.2.3.4/32").unwrap());
         table.remove(cidr_to_ip("1.1.1.2/32").unwrap());

         assert_eq!(table.get_recipient(ip1), Ok(5));

    }



}