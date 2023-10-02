use std::collections::BTreeSet;

use elvis_core::protocols::{
    arp::subnetting::{Ipv4Mask, Ipv4Net},
    ipv4::Ipv4Address,
};

/// A struct used to generate IP addresses and subnets.
#[derive(Debug, Clone)]
pub struct IpGenerator {
    // blocked out IP ranges
    available_ranges: BTreeSet<IpRange>,
}

impl IpGenerator {
    // TODO: currently this implementation is slow if you block out a lot of IPs.
    // It would make my data structures professor sad.
    // Maybe it can be improved later.

    /// Creates a new `IpGenerator` which can generate IPs between the
    /// range's start and end (inclusive).
    pub fn new(range: IpRange) -> Self {
        let mut result = Self::none();
        result.return_range(range);
        result
    }

    /// Creates a new `IpGenerator` which can generate any IP in this subnet.
    pub fn new_sub(net: Ipv4Net) -> Self {
        let range = IpRange::new(net.id(), net.broadcast());
        IpGenerator::new(range)
    }

    /// Creates an `IpGenerator` which can generate any IP in this subnet,
    /// *except* for the first and last IPs in the net
    /// (since these are the IDs and broadcast addresses).
    pub fn new_sub_no_ends(net: Ipv4Net) -> Self {
        let start = add(net.id(), 1);
        let end = add(net.id(), -1);

        // if overflow occured, then there would be no IP addresses in the
        // network, so we can just generate none
        match (start, end) {
            (Some(start), Some(end)) => IpGenerator::new(IpRange::new(start, end)),
            _other => IpGenerator::none(),
        }
    }

    /// Returns an IpGenerator that can generate all IP addresses.
    pub fn all() -> Self {
        Self::new(IpRange::new(
            [0, 0, 0, 0].into(),
            [255, 255, 255, 255].into(),
        ))
    }

    ///Returns an IpGenerator with all IP addresses blocked
    pub fn none() -> Self {
        Self {
            available_ranges: BTreeSet::new(),
        }
    }

    /// Returns an IpGenerator with all reserved IP addresses blocked out.
    ///
    /// <https://en.wikipedia.org/wiki/Reserved_IP_addresses#IPv4>
    pub fn blocked_out() -> Self {
        let mut result = Self::all();
        result.block_reserved_ips();
        result
    }

    /// Takes an existing IpGenerator an blockd all reserved IP addresses
    ///
    /// <https://en.wikipedia.org/wiki/Reserved_IP_addresses#IPv4>
    pub fn block_reserved_ips(&mut self) {
        let mut block = |ip: [u8; 4], mask: u32| {
            self.block_subnet(Ipv4Net::new(
                Ipv4Address::from(ip),
                Ipv4Mask::from_bitcount(mask),
            ));
        };

        block([0, 0, 0, 0], 8);
        block([10, 0, 0, 0], 8);
        block([100, 64, 0, 0], 10);
        block([127, 0, 0, 0], 8);
        block([169, 254, 0, 0], 16);
        block([172, 16, 0, 0], 12);
        block([192, 0, 0, 0], 24);
        block([192, 0, 2, 0], 24);
        block([192, 88, 99, 0], 24);
        block([192, 168, 0, 0], 16);
        block([198, 18, 0, 0], 15);
        block([198, 51, 100, 0], 24);
        block([203, 0, 113, 0], 24);
        block([224, 0, 0, 0], 4);
        block([233, 252, 0, 0], 24);
        block([240, 0, 0, 0], 4);
        block([255, 255, 255, 255], 32);
    }

    /// Generates a single IP address,
    /// then blocks it out so it can't be generated again.
    /// Returns `Some(ip)` if there is an IP address available.
    /// Otherwise returns `None`.
    pub fn fetch_ip(&mut self) -> Option<Ipv4Address> {
        self.fetch_net(Ipv4Mask::from_bitcount(32))
            .map(|net| net.id())
    }

    /// Generates a single network from the available IP addresses,
    /// then blocks it out so it can't be generated again.
    /// Returns `Some(net)` if there is a network available.
    /// Otherwise returns `None`.
    pub fn fetch_net(&mut self, mask: Ipv4Mask) -> Option<Ipv4Net> {
        // check out start of every available range
        // this involves copying, which is not good,
        // but it was pretty hard to satisfy the borrow checker otherwise.
        let ranges: Vec<IpRange> = self.available().collect();
        for av_range in ranges {
            let new_net = match next(av_range.start, mask) {
                Some(net) => net,
                None => continue,
            };

            if av_range.contains(new_net.into()) {
                self.block_subnet(new_net);
                return Some(new_net);
            }
        }
        None
    }

    /// Returns `true` if the network has not been blocked out.
    pub fn is_available(&self, net: Ipv4Net) -> bool {
        !self
            .available_ranges
            .iter()
            .any(|av_range| av_range.contains(net.into()))
    }

    /// Makes IP addresses in a subnet available for generating.
    pub fn return_subnet(&mut self, net: Ipv4Net) {
        self.return_range(net.into())
    }

    fn return_range(&mut self, range: IpRange) {
        self.available_ranges.insert(range);
    }

    /// Makes an IP address available for generating.
    pub fn return_ip(&mut self, returned: Ipv4Address) {
        self.return_subnet(Ipv4Net::new_1(returned))
    }

    /// Prevents a network from being generated.
    pub fn block_subnet(&mut self, network: Ipv4Net) {
        self.block_range(IpRange::from(network));
    }

    fn block_range(&mut self, range: IpRange) {
        // remove all ranges contained
        self.available_ranges
            .retain(|av_range| !range.contains(*av_range));

        // This involves copying, which is bad, but it's hard to appease the borrow checker
        let mut overlapping = Vec::new();
        for av_range in self.available() {
            if av_range.overlaps(range) {
                overlapping.push(av_range);
            }
        }

        for av_range in overlapping {
            self.available_ranges.remove(&av_range);

            // only add left range if overflow would not occur
            if range.start > Ipv4Address::new([0, 0, 0, 0]) {
                let left_end = add(range.start, -1).expect("overflow should be handled");
                let left_range = IpRange::new(av_range.start, left_end);
                if !left_range.is_empty() {
                    self.available_ranges.insert(left_range);
                }
            }

            // only add right range if overflow would not occur
            if range.end < Ipv4Address::new([255, 255, 255, 255]) {
                let right_start = add(range.end, 1).expect("overflow should be handled");
                let right_range = IpRange::new(right_start, av_range.end);
                if !right_range.is_empty() {
                    self.available_ranges.insert(right_range);
                }
            }
        }
    }

    fn available(&self) -> impl DoubleEndedIterator<Item = IpRange> + '_ {
        self.available_ranges.iter().copied()
    }

    /// Returns an iterator over the available networks in this generator
    /// that have a mask of `mask`.
    pub fn into_net_iter(self, mask: Ipv4Mask) -> Ipv4NetIter {
        Ipv4NetIter {
            generator: self,
            mask,
        }
    }

    /// Returns an iterator over the available IP addresses in this generator.
    pub fn into_ip_iter(self) -> Ipv4AddrIter {
        Ipv4AddrIter { generator: self }
    }
}

fn add(ip: Ipv4Address, n: i32) -> Option<Ipv4Address> {
    ip.to_u32().checked_add_signed(n).map(Ipv4Address::from)
}

/// Returns the Ipv4Net starting at the given address
/// If one cannot start at the given address, find the next one
/// Returns none if there is not one (due to overflow)
fn next(ip: Ipv4Address, mask: Ipv4Mask) -> Option<Ipv4Net> {
    let net = Ipv4Net::new(ip, mask);
    if net.id() == ip {
        return Some(net);
    }

    // go to next net
    let ip = add(net.broadcast(), 1)?;
    Some(Ipv4Net::new(ip, mask))
}

/// An inclusive range of IP addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IpRange {
    pub start: Ipv4Address,
    pub end: Ipv4Address,
}

impl IpRange {
    pub fn new(start: Ipv4Address, end: Ipv4Address) -> Self {
        Self { start, end }
    }

    pub fn overlaps(self, other: IpRange) -> bool {
        self.start <= other.end && self.end >= other.start
    }

    pub fn contains(self, other: IpRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn is_empty(self) -> bool {
        self.end < self.start
    }
}

impl From<Ipv4Net> for IpRange {
    fn from(net: Ipv4Net) -> Self {
        Self {
            start: net.id(),
            end: net.broadcast(),
        }
    }
}

impl From<(Ipv4Address, Ipv4Address)> for IpRange {
    fn from(value: (Ipv4Address, Ipv4Address)) -> Self {
        IpRange::new(value.0, value.1)
    }
}

/// A struct which can repeatedly generate networks from an IP generator.
/// Created by [`IpGenerator::into_net_iter`].
pub struct Ipv4NetIter {
    generator: IpGenerator,
    mask: Ipv4Mask,
}

impl Iterator for Ipv4NetIter {
    type Item = Ipv4Net;

    fn next(&mut self) -> Option<Ipv4Net> {
        self.generator.fetch_net(self.mask)
    }
}

/// An iterator through the IP addresses available in an IpGenerator.
/// Created by [`IpGenerator::into_ip_iter`].
pub struct Ipv4AddrIter {
    generator: IpGenerator,
}

impl Iterator for Ipv4AddrIter {
    type Item = Ipv4Address;

    fn next(&mut self) -> Option<Ipv4Address> {
        self.generator.fetch_ip()
    }
}

#[cfg(test)]
mod tests {
    // changing how the generator works may cause these tests to fail in the future.
    // that is fine. just make sure you know what you are doing
    use super::*;
    #[test]
    fn basic() {
        // future calls to fetch_net and fetch_ip may generate IP addresses before previously generated ones,
        // if there is a gap.
        let mut gen = IpGenerator::new(Ipv4Net::new_short([12, 13, 14, 0], 24).into());
        assert_eq!(gen.fetch_ip(), Some(Ipv4Address::new([12, 13, 14, 0])));
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(25)),
            Some(Ipv4Net::new_short([12, 13, 14, 128], 25))
        );
        assert_eq!(gen.fetch_ip(), Some(Ipv4Address::new([12, 13, 14, 1])));
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(26)),
            Some(Ipv4Net::new_short([12, 13, 14, 64], 26))
        );
        assert_eq!(gen.fetch_net(Ipv4Mask::from_bitcount(25)), None);
    }

    /// This checks out edge cases.
    /// I want to make sure that generating IP addresses near 0.0.0.0 and 255.255.255.255 don't break everything.
    #[test]
    fn edge_cases() {
        let mut gen = IpGenerator::blocked_out();
        // block entire internet
        let all_ips = Ipv4Net::new_short([0, 0, 0, 0], 0);
        gen.block_subnet(all_ips);
        assert_eq!(gen.fetch_ip(), None);

        // return entire internet
        gen.return_subnet(all_ips);
        assert_eq!(gen.fetch_ip(), Some([0, 0, 0, 0].into()));

        // block entire internet again
        gen.block_subnet(all_ips);
        gen.return_ip([255, 255, 255, 255].into());
        assert_eq!(gen.fetch_ip(), Some([255, 255, 255, 255].into()));

        // block a random subnet
        gen.return_subnet(all_ips);
        gen.block_subnet(Ipv4Net::new_short([12, 13, 14, 15], 8));
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(16)),
            Some(Ipv4Net::new_short([0, 0, 0, 0], 16))
        );
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(1)),
            Some(Ipv4Net::new_short([128, 0, 0, 0], 1))
        );
    }

    /// A test that involves repeatedly blocking and making available IP addresses.
    #[test]
    fn throttle() {
        let mut gen = IpGenerator::blocked_out();
        // should be a no-op
        gen.return_subnet(Ipv4Net::new_short([1, 0, 0, 0], 24));
        let res = gen.fetch_net(Ipv4Mask::from_bitcount(24)).unwrap();

        // generate 2 24-mask subnets
        assert_eq!(res, Ipv4Net::new_short([1, 0, 0, 0], 24));
        let res2 = gen.fetch_net(Ipv4Mask::from_bitcount(24)).unwrap();
        assert_eq!(res2, Ipv4Net::new_short([1, 0, 1, 0], 24));
        gen.return_subnet(res);

        let current_network = Ipv4Net::new_short([0, 0, 0, 0], 8);
        gen.return_subnet(current_network);
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(8)).unwrap(),
            current_network
        );

        // generate 2 more
        assert_eq!(gen.fetch_net(Ipv4Mask::from_bitcount(24)).unwrap(), res);
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(24)).unwrap(),
            Ipv4Net::new_short([1, 0, 2, 0], 24)
        );

        assert_eq!(gen.fetch_ip().unwrap(), [1, 0, 3, 0].into());
        gen.return_subnet(res2);
        assert_eq!(
            gen.fetch_net(Ipv4Mask::from_bitcount(16)).unwrap(),
            Ipv4Net::new_short([1, 1, 0, 0], 16)
        );
    }
}
