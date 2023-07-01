use std::collections::BTreeMap;

use crate::protocols::ipv4::Recipient;
use crate::protocols::ipv4::Ipv4Address;
use crate::protocols::arp::subnetting::*;

type Entry = (Ipv4Address, Ipv4Mask);

pub struct IpTable {
    table: BTreeMap<Entry, Recipient>,
    masks: BTreeMap<Ipv4Mask, u32>
}

impl IpTable {
    pub fn new() -> Self {
        IpTable {
            table: Default::default(),
            masks: Default::default()
        }
    }

    pub fn default_gateway(recipient: Recipient) -> Self {
        let mut table = IpTable::new();
        table.add( cidr_to_ip("0.0.0.0/0").unwrap(), recipient);
        table
    }

    // get recipient associated with provided ipv4 address.
    // searches through list of masks and checks if the masked ip and mask pair
    // is on the table and returns the first matching recipient
    pub fn get_recipient(&mut self, address: Ipv4Address) -> Result<Recipient, ()> {
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

    pub fn add(&mut self, key: Entry, value: Recipient) {
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
    fn setup() -> IpTable {
        let mut table = IpTable::new();

        table.add(cidr_to_ip("1.0.0.0/8").unwrap(), Recipient::new(20, None));
        table.add(cidr_to_ip("1.1.0.0/16").unwrap(), Recipient::new(0, None));
        table.add(cidr_to_ip("1.1.1.0/24").unwrap(), Recipient::new(1, None));
        table.add(cidr_to_ip("1.1.2.0/24").unwrap(), Recipient::new(2, None));
        table.add(cidr_to_ip("1.1.3.0/24").unwrap(), Recipient::new(3, None));
        table.add(cidr_to_ip("1.2.3.0/24").unwrap(), Recipient::new(4, None));
        table.add(cidr_to_ip("1.1.1.2/32").unwrap(), Recipient::new(5, None));
        table.add(cidr_to_ip("1.2.3.4/32").unwrap(), Recipient::new(6, None));

        table
    }

    #[test]
    fn test_add() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1,1,1,2]);
        let ip2 = Ipv4Address::new([1,1,0,1]);

        assert_eq!(table.get_recipient(ip1), Ok(Recipient::new(5, None)));
        assert_eq!(table.get_recipient(ip2), Ok(Recipient::new(0, None)));

        table.add(cidr_to_ip("1.1.0.0/24").unwrap(), Recipient::new(20, None));

        assert_eq!(table.get_recipient(ip2), Ok(Recipient::new(20, None)));
    }

    #[test]
    fn test_remove() {
         let mut table = setup();

         let ip1 = Ipv4Address::new([1,2,3,4]);

         assert_eq!(table.get_recipient(ip1), Ok(Recipient::new(6, None)));

         table.remove(cidr_to_ip("1.2.3.4/32").unwrap());

         assert_eq!(table.get_recipient(ip1), Ok(Recipient::new(4, None)));

         print_table(&table);
    }

    #[allow(dead_code)]
    pub fn print_table(table: &IpTable) {
        for entry in table.table.iter() {
            println!("{:?}", entry);
        }

        println!();

        for entry in table.masks.iter() {
            println!("{:?}", entry);
        }
    }


}