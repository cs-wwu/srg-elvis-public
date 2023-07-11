use std::collections::BTreeMap;
use std::fmt;

use crate::machine::PciSlot;
use crate::protocols::arp::subnetting::*;
use crate::protocols::ipv4::{Ipv4Address, Recipient, Recipients};
use std::collections::btree_map::*;

use std::fmt::Debug;

type Entry = (Ipv4Address, Ipv4Mask);

/// An IpTable is a type of map that maps (Ipv4, Ipv4Mask) to the given type T
/// this mapping is different from a traditional HashMap/TreeMap in a sense
/// that entries are accsessed by providing a single ipv4address.
/// When the ipv4 address is provided the table starts with the highest
/// mask on the table and applies it to the provided ipv4address then
/// checks if the masked ipv4address, mask pair is in the table
#[derive(Eq, PartialEq)]
pub struct IpTable<T: Copy> {
    table: BTreeMap<Entry, T>,
    // mapping to keep track of number of num of unique subnets associated with
    // each mask
    masks: BTreeMap<Ipv4Mask, u32>,
}

// TODO (eulerfrog) add examples for each fn
impl<T: Copy> IpTable<T> {
    pub fn new() -> Self {
        IpTable {
            table: Default::default(),
            masks: Default::default(),
        }
    }

    /// Specifies the default recipient to send packets to if no other subnet
    /// is found in the table.
    pub fn default_gateway(recipient: T) -> Self {
        let mut table = IpTable::new();
        table.add(cidr_to_ip("0.0.0.0/0").unwrap(), recipient);
        table
    }

    /// Gets value associated with provided ipv4 address by starting at the largest bit mask and
    /// returns the destination associated with that mask. If no subnet is found,
    /// the recipient linked to the default gateway is returned. If no default gateway is
    /// then None is returned
    pub fn get_recipient(&self, address: Ipv4Address) -> Option<T> {
        for entry in self.masks.keys().rev() {
            let masked_address = get_network_id(address, *entry);

            if let Some(recipient) = self.table.get(&(masked_address, *entry)) {
                return Some(*recipient);
            }
        }
        None
    }

    /// Removes subnet associated with given key from the table
    pub fn remove(&mut self, key: Entry) {
        let masked_key = (get_network_id(key.0, key.1), key.1);

        match self.table.remove(&masked_key) {
            None => return,
            Some(_) => {}
        }

        if let Some(&val) = self.masks.get(&key.1) {
            if val == 1 {
                self.masks.remove(&key.1);
            } else {
                self.masks.insert(key.1, val - 1);
            }
        }
    }

    /// Removes address associated with given ip address using
    /// 32 bit mask length as second part of the key.
    pub fn remove_direct(&mut self, address: Ipv4Address) {
        self.remove((address, Ipv4Mask::from_bitcount(32)));
    }

    /// Removes address associated with given ip address using
    /// cidr notation. If notation is invalid the table
    /// is left unchanged
    pub fn remove_cidr(&mut self, cidr: &str) {
        if let Ok(key) = cidr_to_ip(cidr) {
            self.remove(key);
        }
    }

    /// Maps subnet associated with (Ipv4Address, Ipv4Mask) pair
    /// to provided value.
    pub fn add(&mut self, key: Entry, value: T) {
        let masked_key = (get_network_id(key.0, key.1), key.1);

        // if we replaced an entry in the table don't update the mask count
        if self.table.insert(masked_key, value).is_some() {
            return;
        }

        let total = match self.masks.get(&key.1) {
            Some(val) => *val,
            None => 0,
        };

        self.masks.insert(key.1, total + 1);
    }

    /// Maps ipv4 address associated with the subnet: address/32
    /// to provided value.
    pub fn add_direct(&mut self, address: Ipv4Address, value: T) {
        self.add((address, Ipv4Mask::from_bitcount(32)), value);
    }

    /// Maps subnet associated with given cidr notation to
    /// to provided value.
    pub fn add_cidr(&mut self, cidr: &str, value: T) {
        if let Ok(key) = cidr_to_ip(cidr) {
            self.add(key, value);
        }
    }

    pub fn iter(&self) -> Iter<'_, (Ipv4Address, Ipv4Mask), T> {
        self.table.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, (Ipv4Address, Ipv4Mask), T> {
        self.table.iter_mut()
    }
}

/// Allows conversion of Recipients into IpTable
///
impl From<Recipients> for IpTable<Recipient> {
    fn from(other: Recipients) -> Self {
        let mut table = Self::new();
        for pair in other.iter() {
            table.add_direct(*pair.0, *pair.1);
        }
        table
    }
}

impl<T: Copy> Clone for IpTable<T> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            masks: self.masks.clone(),
        }
    }
}

/// Allows creation of an ip table from ((Ipv4Adress, Ipv4Mask), T) pairs
///
impl<T: Copy> FromIterator<((Ipv4Address, Ipv4Mask), T)> for IpTable<T> {
    fn from_iter<I: IntoIterator<Item = ((Ipv4Address, Ipv4Mask), T)>>(iter: I) -> Self {
        let mut table = Self::new();
        for pair in iter {
            table.add(pair.0, pair.1);
        }
        table
    }
}

/// Allows creation of a table from an iterator of (Ipv4Adress, T)
///
impl<T: Copy> FromIterator<(Ipv4Address, T)> for IpTable<T> {
    fn from_iter<I: IntoIterator<Item = (Ipv4Address, T)>>(iter: I) -> Self {
        let mut table = Self::new();
        for pair in iter {
            table.add_direct(pair.0, pair.1);
        }
        table
    }
}

/// Allows creation of a table from an into iterator of string slices
/// each string slice must be a valid cidr string to be added to the table
///
impl<'a, T: Copy> FromIterator<(&'a str, T)> for IpTable<T> {
    fn from_iter<I: IntoIterator<Item = (&'a str, T)>>(iter: I) -> Self {
        let mut table = Self::new();
        for pair in iter {
            table.add_cidr(pair.0, pair.1);
        }
        table
    }
}

impl From<IpTable<(Ipv4Address, PciSlot)>> for IpTable<(Ipv4Address, PciSlot, u32, bool)> {
    fn from(other: IpTable<(Ipv4Address, PciSlot)>) -> IpTable<(Ipv4Address, PciSlot, u32, bool)> {
        let mut table = IpTable::new();
        for entry in other.iter() {
            table.add(*entry.0, (entry.1 .0, entry.1 .1, 1, false));
        }
        table
    }
}

impl<T: Copy> Default for IpTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Debug> Debug for IpTable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in self.table.iter() {
            writeln!(
                f,
                "{}/{} : {:?}",
                entry.0 .0,
                entry.0 .1.count_ones(),
                entry.1
            )
            .unwrap();
        }
        Ok(())
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
        table.add_cidr("1.2.3.4/32", 7);

        table
    }

    #[test]
    fn test_add() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1, 1, 1, 2]);
        let ip2 = Ipv4Address::new([1, 1, 0, 1]);

        assert_eq!(table.get_recipient(ip1), Some(6));
        assert_eq!(table.get_recipient(ip2), Some(1));

        table.add(cidr_to_ip("1.1.0.0/24").unwrap(), 20);

        assert_eq!(table.get_recipient(ip2), Some(20));
    }

    #[test]
    fn test_remove() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1, 2, 3, 4]);

        assert_eq!(table.get_recipient(ip1), Some(7));

        table.remove(cidr_to_ip("1.2.3.4/32").unwrap());

        assert_eq!(table.get_recipient(ip1), Some(5));

        table.remove(cidr_to_ip("1.1.1.2/32").unwrap());

        // all 32 bit mask addresses should now be removed from the table
        assert_eq!(table.masks.get(&Ipv4Mask::from_bitcount(32)), None);
    }

    #[test]
    fn test_into() {
        let ip_table: Recipients = [
            (Ipv4Address::new([1, 1, 1, 1]), Recipient::new(0, None)),
            (Ipv4Address::new([1, 1, 1, 2]), Recipient::new(1, None)),
            (Ipv4Address::new([1, 1, 1, 3]), Recipient::new(2, None)),
            (Ipv4Address::new([1, 1, 1, 4]), Recipient::new(3, None)),
        ]
        .into_iter()
        .collect();

        let new_table: IpTable<Recipient> = ip_table.clone().into();

        for ip in ip_table.keys() {
            assert_eq!(
                new_table.get_recipient(*ip).unwrap(),
                *ip_table.get(ip).unwrap()
            );
        }
    }

    #[test]
    fn test_into_iter() {
        let ip_table: IpTable<Recipient> = [
            ("0.0.0.0/0", Recipient::new(0, None)),
            ("1.0.0.0/8", Recipient::new(0, None)),
            ("1.1.0.0/16", Recipient::new(0, None)),
            ("1.1.1.0/24", Recipient::new(0, None)),
            ("1.1.1.1/32", Recipient::new(0, None)),
        ]
        .into_iter()
        .collect();

        println!("{:?}", ip_table);
    }
}
