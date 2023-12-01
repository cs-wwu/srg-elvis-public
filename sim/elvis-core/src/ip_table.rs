use crate::machine::PciSlot;
use crate::protocols::arp::subnetting::*;
use crate::protocols::ipv4::{Ipv4Address, Recipient, Recipients};
use std::any::type_name;
use std::any::TypeId;
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;

/// An IpTable is a type of map that maps (Ipv4, Ipv4Mask) to the given type T
/// this mapping is different from a traditional HashMap/TreeMap in a sense
/// that entries are accsessed by providing a single ipv4address.
/// When the ipv4 address is provided the table starts with the highest
/// mask on the table and applies it to the provided ipv4address then
/// checks if the masked ipv4address, mask pair is in the table
#[derive(Eq, PartialEq)]

pub struct IpTable<T> {
    table: BTreeMap<Obm, T>,
}

// TODO (eulerfrog) add examples for each fn
impl<T: Copy> IpTable<T> {
    pub fn new() -> Self {
        IpTable {
            table: Default::default(),
        }
    }

    /// Specifies the default recipient to send packets to if no other subnet
    /// is found in the table.
    pub fn default_gateway(recipient: T) -> Self {
        let mut table = IpTable::new();
        table.add(Ipv4Net::from_cidr("0.0.0.0/0").unwrap(), recipient);
        table
    }

    /// Gets value associated with provided ipv4 address by starting at the largest bit mask and
    /// returns the destination associated with that mask. If no subnet is found,
    /// the recipient linked to the default gateway is returned. If no default gateway is
    /// then None is returned
    pub fn get_recipient(&self, address: Ipv4Address) -> Option<T> {
        for (net, value) in self.iter() {
            let net = net;
            if net.contains(address) {
                return Some(value);
            }
        }
        None
    }

    /// Removes subnet associated with given key from the table
    pub fn remove(&mut self, key: Ipv4Net) -> Option<T> {
        self.table.remove(&Obm(key))
    }

    /// Removes address associated with given ip address using
    /// 32 bit mask length as second part of the key.
    pub fn remove_direct(&mut self, address: Ipv4Address) -> Option<T> {
        self.remove(Ipv4Net::new(address, Ipv4Mask::from_bitcount(32)))
    }

    /// Removes address associated with given ip address using
    /// cidr notation. Panics if notation is invalid.
    pub fn remove_cidr(&mut self, cidr: &str) {
        let net = Ipv4Net::from_cidr(cidr).expect("CIDR string formatted incorrectly");
        self.remove(net);
    }

    /// Maps subnet associated with (Ipv4Address, Ipv4Mask) pair
    /// to provided value.
    ///
    /// If this net was already present in the map, return the old value.
    pub fn add(&mut self, key: Ipv4Net, value: T) -> Option<T> {
        self.table.insert(Obm(key), value)
    }

    /// Maps ipv4 address associated with the subnet: address/32
    /// to provided value.
    pub fn add_direct(&mut self, address: Ipv4Address, value: T) {
        self.add(Ipv4Net::new(address, Ipv4Mask::from_bitcount(32)), value);
    }

    /// Maps subnet associated with given cidr notation to
    /// to provided value.
    pub fn add_cidr(&mut self, cidr: &str, value: T) {
        if let Ok(key) = Ipv4Net::from_cidr(cidr) {
            self.add(key, value);
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Ipv4Net, T)> + '_ {
        self.table.iter().map(|(net, value)| (net.0, *value))
    }

    // pub fn iter_mut(&mut self) -> IterMut<Obm, T> {
    //     self.table.iter_mut()
    // }
}

#[derive(Clone, Copy, Debug)]
pub struct Rte {
    pub next_hop: Option<Ipv4Address>,
    pub mask: Ipv4Mask,
    pub slot: PciSlot,
    pub metric: u32,
}

impl Rte {
    pub fn new(
        next_hop: Option<Ipv4Address>,
        mask: Ipv4Mask,
        slot: PciSlot,
        metric: u32,
    ) -> Self {
        Self {
            next_hop,
            mask,
            slot,
            metric,
        }
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
        }
    }
}

/// Allows creation of an ip table from ((Ipv4Adress, Ipv4Mask), T) pairs
///
impl<T: Copy> FromIterator<((Ipv4Address, Ipv4Mask), T)> for IpTable<T> {
    fn from_iter<I: IntoIterator<Item = ((Ipv4Address, Ipv4Mask), T)>>(iter: I) -> Self {
        iter.into_iter()
            .map(|entry| (Ipv4Net::from(entry.0), entry.1))
            .collect()
    }
}

impl<T: Copy> FromIterator<(Ipv4Net, T)> for IpTable<T> {
    fn from_iter<I: IntoIterator<Item = (Ipv4Net, T)>>(iter: I) -> Self {
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

impl From<IpTable<(Option<Ipv4Address>, PciSlot)>> for IpTable<Rte> {
    fn from(other: IpTable<(Option<Ipv4Address>, PciSlot)>) -> IpTable<Rte> {
        let mut table = IpTable::new();
        for entry in other.iter() {
            table.add(entry.0, Rte::new(entry.1 .0, entry.0.mask(), entry.1 .1, 1));
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
        for (net, value) in self.table.iter() {
            writeln!(
                f,
                "{}/{} : {:?}",
                net.0.id(),
                net.0.mask().count_ones(),
                value
            )
            .unwrap();
        }
        Ok(())
    }
}

/// OBM stands for "order by mask."
/// This is a wrapper around an Ipv4Net, causing it to be ordered by its mask (greatest to least),
/// then by its IP address.
/// In other words, it orders by network size from least to greatest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Obm(Ipv4Net);

impl PartialOrd for Obm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Obm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // compare masks
        match self.0.mask().cmp(&other.0.mask()) {
            std::cmp::Ordering::Equal => {
                // order by ip address
                self.0.id().cmp(&other.0.id())
            }
            // sort by mask greatest to least
            other_ord => other_ord.reverse(),
        }
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

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

        table.add(Ipv4Net::from_cidr("1.1.0.0/24").unwrap(), 20);

        assert_eq!(table.get_recipient(ip2), Some(20));
    }

    #[test]
    fn test_remove() {
        let mut table = setup();

        let ip1 = Ipv4Address::new([1, 2, 3, 4]);

        assert_eq!(table.get_recipient(ip1), Some(7));

        table.remove(Ipv4Net::from_cidr("1.2.3.4/32").unwrap());

        assert_eq!(table.get_recipient(ip1), Some(5));

        table.remove(Ipv4Net::from_cidr("1.1.1.2/32").unwrap());

        // all 32 bit mask addresses should now be removed from the table
        assert!(!table
            .iter()
            .any(|(net, _value)| net.mask() == Ipv4Mask::from_bitcount(32)));
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

    #[test]
    #[rustfmt::skip]
    fn test_obm() {
        // these networks are in sorted order
        // like how Obm is supposed to work
        let nets = [
            // little network
            Obm(Ipv4Net::from_cidr("35.0.1.0/32").unwrap()),
            // random network
            Obm(Ipv4Net::from_cidr("193.14.26.9/17").unwrap()),
            // big network
            Obm(Ipv4Net::from_cidr("15.0.7.0/16").unwrap()),
            // biggest, first network
            Obm(Ipv4Net::from_cidr("13.0.1.0/8").unwrap()),
            // biggest, second network
            Obm(Ipv4Net::from_cidr("15.0.1.0/8").unwrap()),
            // biggest, third network
            Obm(Ipv4Net::from_cidr("19.0.1.0/8").unwrap()),
        ];

        let mut shuffled_nets = nets.clone();
        shuffled_nets.shuffle(&mut StdRng::seed_from_u64(1234));

        println!("shuffled:");
        for net in &shuffled_nets {
            println!("{:?}", net);
        }

        // test to make sure that ip table's iterator gives things in the correct order
        let mut ip_table = IpTable::new();
        for obm in &shuffled_nets {
            ip_table.add(obm.0, ());
        }
        let nets_no_obm = nets.iter().map(|obm| obm.0);
        let table_keys = ip_table.iter().map(|(net, _v)| net);
        assert!(nets_no_obm.eq(table_keys));

        // test to make sure that sorting works
        shuffled_nets.sort();
        assert_eq!(shuffled_nets, nets);
    }
}
