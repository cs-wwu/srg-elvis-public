//! Contains structs and functions needed to do Ipv4 subnetting in ELVIS.
//!
//! While subnetting is not usually considered to be part of ARP,
//! ELVIS is organized in this way because [`Arp`](super::Arp) is in charge of making sure that
//! packets get sent to the default gateway's MAC address.
//!
//! For more info, see [`Arp`](super::Arp) and `Arp::open`.
//!
//! # Usage of CIDR
//!
//! ELVIS uses CIDR (classless inter-domain routing) subnetting.
//! It does not support "classful networks" (e.g., type A/B/C/D/E networks).
//!
//! Wikipedia article on CIDR: <https://en.wikipedia.org/wiki/Classless_Inter-Domain_Routing>
//!
//! # Support of reserved IPs
//!
//! This code is largely informed by [PN YouTube videos](https://youtu.be/BWZ-MHIhqjM)
//! and Wikipedia articles, not by official RFCs. As such, some of these functions
//! may be oversimplified. Many of the special reserved IPs are not currently supported by ELVIS.

use std::{net::Ipv4Addr, ops::RangeInclusive, str::FromStr};

use crate::protocols::ipv4::Ipv4Address;

/// A struct representing an Ipv4 subnet mask.
/// (It's a thin wrapper around a u32.)
#[derive(Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Ipv4Mask(u32);

// const version of clamp function
const fn clamp(num: u32, min: u32, max: u32) -> u32 {
    assert!(min <= max);
    if num < min {
        min
    } else if num > max {
        max
    } else {
        num
    }
}

impl std::fmt::Debug for Ipv4Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Ipv4Mask")
            .field(&Ipv4Address::from(self.0))
            .finish()
    }
}

impl std::fmt::Display for Ipv4Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Ipv4Mask {
    /// Returns a mask of `size` 1s.
    /// Should be a number from 0 to 32.
    /// If `size > 32`, then it will be set to 32.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// // 255.255.0.0
    /// let mask = Ipv4Mask::from_bitcount(16);
    /// assert_eq!(u32::from(mask), 0xFF_FF_00_00);
    /// ```
    pub const fn from_bitcount(size: u32) -> Ipv4Mask {
        let size = clamp(size, 0, 32);
        if size == 0 {
            Ipv4Mask(0)
        } else if size == 32 {
            Ipv4Mask(0xFF_FF_FF_FF)
        } else {
            Ipv4Mask(((1 << size) - 1) << (32 - size))
        }
    }

    /// Returns the number of 1s in this mask.
    /// For example, the mask 0xFF_FF_FF_00 (or 255.255.255.0)
    /// has 24 1s.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// let mask = Ipv4Mask::from_bitcount(9);
    /// assert_eq!(mask.count_ones(), 9);
    /// ```
    pub const fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    /// Turns the mask into a u32.
    pub const fn to_u32(self) -> u32 {
        self.0
    }

    /// Turns the mask into an Ipv4 address.
    pub const fn to_ipv4_address(self) -> Ipv4Address {
        Ipv4Address::new(self.to_u32().to_be_bytes())
    }

    /// Returns the number of IP addresses that a network using this mask would have.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// // A 4-bit host portion allows for 2^4 or 16 IP addresses
    /// // including network ID and broadcast IP
    /// let ips = Ipv4Mask::from_bitcount(28).ips_in_net();
    /// assert_eq!(ips, 16);
    /// ```
    pub const fn ips_in_net(&self) -> u64 {
        let wildcard = !(self.to_u32());
        let wildcard = wildcard as u64;
        wildcard + 1
    }

    /// Returns the number of usable addresses that must be in a network based on the subnet mask.
    /// (This does not include the broadcast or ID of each network.)
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// // A 4-bit host portion allows for 2^4 - 2 or 14 IP addresses
    /// // (the ID and broadcast IP of the network are not included)
    /// let ips = Ipv4Mask::from_bitcount(28).usable_ips();
    /// assert_eq!(ips, 14);
    /// ```
    pub const fn usable_ips(&self) -> u32 {
        let wildcard = !(self.to_u32());
        match wildcard {
            0 | 1 => 0,
            other => other - 1,
        }
    }
}

impl From<Ipv4Mask> for u32 {
    fn from(mask: Ipv4Mask) -> u32 {
        mask.0
    }
}

impl From<Ipv4Mask> for Ipv4Address {
    fn from(mask: Ipv4Mask) -> Ipv4Address {
        Ipv4Address::from(mask.to_u32())
    }
}

impl TryFrom<u32> for Ipv4Mask {
    type Error = u32;

    /// Returns an Ipv4Mask based on the u32.
    /// If the u32 is not a valid subnet mask (that is, it has 0s between the 1s),
    /// this will return the number back as an error.
    fn try_from(mask: u32) -> Result<Ipv4Mask, u32> {
        let count = mask.count_ones();
        let result = Ipv4Mask::from_bitcount(count);
        if u32::from(result) == mask {
            Ok(result)
        } else {
            Err(mask)
        }
    }
}

impl TryFrom<Ipv4Address> for Ipv4Mask {
    type Error = Ipv4Address;

    /// Returns an Ipv4Mask based on the Ipv4Address.
    /// If the Ipv4Address is not a valid subnet mask (that is, it has 0s between the 1s),
    /// this will return the address back as an error.
    fn try_from(mask: Ipv4Address) -> Result<Ipv4Mask, Ipv4Address> {
        Ipv4Mask::try_from(mask.to_u32()).or(Err(mask))
    }
}

/// `Ipv4Net` stands for "Ipv4 Network ID". It is a struct
/// containing an `Ipv4Address` and an `Ipv4Mask`.
/// Can be used to identify a network using, of course, an IP address and mask.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv4Net {
    /// This MUST be a network ID or it will screw up Eq
    network_id: Ipv4Address,
    mask: Ipv4Mask,
}

impl Ipv4Net {
    /// Creates an Ipv4Net from an IP address and mask.
    pub fn new(ip: Ipv4Address, mask: Ipv4Mask) -> Self {
        Self {
            network_id: Ipv4Address::from(ip.to_u32() & mask.to_u32()),
            mask,
        }
    }

    /// Creates a new Ipv4Net from something that can be converted into
    /// an IP address, and a mask length.
    /// The mask length is clamped to the 0-32 range.
    ///
    /// # Example
    ///
    /// ```
    /// # use elvis_core::protocols::ipv4::*;
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// let net1 = Ipv4Net::new_short([12, 13, 12, 0], 28);
    /// let net2 = Ipv4Net::new(Ipv4Address::from([12, 13, 12, 0]), Ipv4Mask::from_bitcount(28));
    /// assert_eq!(net1, net2);
    /// ```
    pub fn new_short(ip: impl Into<Ipv4Address>, mask_len: u32) -> Ipv4Net {
        Ipv4Net::new(ip.into(), Ipv4Mask::from_bitcount(mask_len))
    }

    /// Creates an Ipv4Net containing a single IP address.
    pub fn new_1(ip: Ipv4Address) -> Self {
        Self {
            network_id: ip,
            mask: Ipv4Mask::from_bitcount(32),
        }
    }

    /// Turns an string in [CIDR notation](https://en.wikipedia.org/wiki/Classless_Inter-Domain_Routing#CIDR_notation)
    /// into a `Ipv4NetworkId`.
    ///
    /// Returns an error if the string is not of form `ip_address/mask_length`.
    ///
    /// See [`cidr_to_ip`] for a function that directly returns an IP and mask.
    pub fn from_cidr(cidr: &str) -> Result<Ipv4Net, CidrParseError> {
        cidr_to_ip(cidr).map(Ipv4Net::from)
    }

    /// Returns the first IP address in this network.
    ///
    /// # Example
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// # use elvis_core::protocols::ipv4::*;
    /// let net = Ipv4Net::from_cidr("10.0.0.119/24").unwrap();
    /// let broadcast = net.id();
    /// assert_eq!(broadcast, Ipv4Address::from([10,0,0,0]));
    /// ```
    pub fn id(&self) -> Ipv4Address {
        self.network_id
    }

    /// Returns the broadcast IP address for this network.
    /// (This is the last IP address in the network.)
    ///
    /// # Example
    ///
    /// ```
    /// # use elvis_core::protocols::arp::subnetting::*;
    /// # use elvis_core::protocols::ipv4::*;
    /// let net = Ipv4Net::from_cidr("10.0.0.119/24").unwrap();
    /// let broadcast = net.broadcast();
    /// assert_eq!(broadcast, Ipv4Address::from([10,0,0,255]));
    /// ```
    pub fn broadcast(&self) -> Ipv4Address {
        let ip_id = self.id();
        let new_ip_u32 = ip_id.to_u32() + (!self.mask.to_u32());
        Ipv4Address::new(new_ip_u32.to_be_bytes())
    }

    /// Returns the `Ipv4Mask` of this network.
    pub fn mask(&self) -> Ipv4Mask {
        self.mask
    }

    /// Converts this network to a range of IP addresses.
    pub fn range(&self) -> RangeInclusive<Ipv4Address> {
        self.id()..=self.broadcast()
    }

    /// Returns `true` if the `address` is contained in this network.
    pub fn contains(&self, address: Ipv4Address) -> bool {
        self.id().to_u32() == address.to_u32() & self.mask().to_u32()
    }

    /// Returns `true` if these 2 networks contain overlapping IP addresses.
    pub fn overlaps(&self, other: Self) -> bool {
        self.id() <= other.broadcast() && self.broadcast() >= other.id()
    }
}

impl From<(Ipv4Address, Ipv4Mask)> for Ipv4Net {
    fn from(value: (Ipv4Address, Ipv4Mask)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<Ipv4Net> for (Ipv4Address, Ipv4Mask) {
    /// Converts an `Ipv4NetworkId` to an `(Ipv4Address, Ipv4Mask)`.
    /// The `Ipv4Address` is the network ID obtained by
    /// [`Ipv4Net::id`].
    fn from(value: Ipv4Net) -> Self {
        (value.id(), value.mask())
    }
}

impl TryFrom<RangeInclusive<Ipv4Address>> for Ipv4Net {
    type Error = TryFromRangeError;

    /// Creates a RangeInclusive of Ipv4Addresses into an Ipv4NetworkId.
    /// Returns an error if the given range of IPs could not be represented
    /// by an IP and subnet mask.
    fn try_from(value: RangeInclusive<Ipv4Address>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(TryFromRangeError::Empty);
        }

        // The mask is actually the `not` of the number of values in the range - 1. pretty neat
        let mask = !(value.end().to_u32() - value.start().to_u32());
        let mask = Ipv4Mask::try_from(mask).or(Err(TryFromRangeError::Size))?;
        let result = Ipv4Net::new(*value.start(), mask);
        if result.range() == value {
            Ok(result)
        } else {
            Err(TryFromRangeError::Start)
        }
    }
}

impl std::fmt::Debug for Ipv4Net {
    /// The results will be of form `Ipv4Net {10.0.0.23/8}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Ipv4Net {{{}/{}}}",
            self.network_id,
            self.mask().count_ones()
        ))
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum TryFromRangeError {
    #[error("range is empty")]
    Empty,
    #[error("range's size was not a power of 2, could not make subnet mask")]
    Size,
    #[error("a network cannot start and end in those positions")]
    Start,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("Failed to parse CIDR string")]
pub enum CidrParseError {
    Ipv4,
    Mask(#[from] std::num::ParseIntError),
}

/// A struct containing data needed for subnetting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct SubnetInfo {
    /// The subnet mask of this machine's network.
    pub mask: Ipv4Mask,
    /// The default gateway.
    pub default_gateway: Ipv4Address,
}

impl SubnetInfo {
    /// Creates a SubnetInfo.
    pub const fn new(mask: Ipv4Mask, default_gateway: Ipv4Address) -> SubnetInfo {
        SubnetInfo {
            mask,
            default_gateway,
        }
    }
}

/// Turns an string in [CIDR notation](https://en.wikipedia.org/wiki/Classless_Inter-Domain_Routing#CIDR_notation)
/// into an Ipv4 address and a subnet mask.
///
/// Returns an error if the string is not of form `ip_address/mask_length`.
///
/// See [`Ipv4Net::from_cidr`] if you would like to create a network directly.
///
/// # Examples
///
/// ```
/// # use elvis_core::protocols::arp::subnetting::*;
/// # use elvis_core::protocols::ipv4::Ipv4Address;
/// let (ip, mask) = cidr_to_ip("123.45.67.8/14").unwrap();
/// assert_eq!(ip, Ipv4Address::new([123, 45, 67, 8]));
/// assert_eq!(mask, Ipv4Mask::from_bitcount(14));
///
/// let result = cidr_to_ip("5.6.7.8");
/// result.expect_err("5.6.7.8 is just an IP address, not CIDR notation!");
/// ```
pub fn cidr_to_ip(cidr: &str) -> Result<(Ipv4Address, Ipv4Mask), CidrParseError> {
    let mut parts = cidr.split('/');
    let mut next = || parts.next().ok_or(CidrParseError::Ipv4);
    let ip_str = next()?;
    let mask_str = next()?;

    let ip = Ipv4Addr::from_str(ip_str)
        .or(Err(CidrParseError::Ipv4))?
        .octets()
        .into();
    let mask = Ipv4Mask::from_bitcount(u32::from_str(mask_str)?);
    Ok((ip, mask))
}

#[cfg(test)]
mod tests {
    use crate::protocols::ipv4::Ipv4Address;

    use super::{Ipv4Mask, Ipv4Net, TryFromRangeError};

    // hehe ipad
    type Ipad = Ipv4Address;
    type Nid = Ipv4Net;

    #[test]
    fn ipv4_network_id() {
        let ip = Ipad::new([67, 2, 3, 4]);
        let mask = Ipv4Mask::from_bitcount(8);
        let start = Ipad::new([67, 0, 0, 0]);
        let end = Ipad::new([67, 255, 255, 255]);
        let net = Nid::new(ip, mask);

        assert_eq!(net.id(), start);
        assert_eq!(net.broadcast(), end);
        assert_eq!(net.mask(), mask);

        assert!(net.contains([67, 0, 0, 0].into()));
        assert!(net.contains([67, 255, 255, 255].into()));
        assert!(net.contains([67, 2, 17, 17].into()));
        assert!(!net.contains([66, 255, 255, 255].into()));
        assert!(!net.contains([68, 0, 0, 0].into()));

        assert_eq!(net.range(), start..=end);
    }

    #[test]
    fn try_from_range() {
        let net1 = Nid::new([67, 0, 0, 0].into(), Ipv4Mask::from_bitcount(8));
        let range1 = Ipad::new([67, 0, 0, 0])..=Ipad::new([67, 255, 255, 255]);
        assert_eq!(range1.try_into(), Ok(net1));

        let net2 = Nid::new([0, 0, 0, 0].into(), Ipv4Mask::from_bitcount(0));
        let range2 = Ipad::new([0, 0, 0, 0])..=Ipad::new([255, 255, 255, 255]);
        assert_eq!(range2.try_into(), Ok(net2));

        let net3 = Nid::new([12, 13, 14, 15].into(), Ipv4Mask::from_bitcount(32));
        let range3 = Ipad::new([12, 13, 14, 15])..=Ipad::new([12, 13, 14, 15]);
        assert_eq!(range3.try_into(), Ok(net3));

        let range4 = Ipad::new([17, 1, 2, 3])..=Ipad::new([13, 1, 2, 3]);
        assert_eq!(Nid::try_from(range4), Err(TryFromRangeError::Empty));

        let range5 = Ipad::new([45, 0, 0, 0])..=Ipad::new([45, 0, 0, 17]);
        assert_eq!(Nid::try_from(range5), Err(TryFromRangeError::Size));

        let range6 = Ipad::new([45, 0, 0, 129])..=Ipad::new([45, 0, 0, 132]);
        assert_eq!(Nid::try_from(range6), Err(TryFromRangeError::Start));

        // random overlap tests I'm throwing in for convenience sake
        assert!(net1.overlaps(net2));
        assert!(net2.overlaps(net1));
        assert!(!net3.overlaps(net1));
        assert!(net1.overlaps(Nid::new(
            [67, 255, 255, 252].into(),
            Ipv4Mask::from_bitcount(2)
        )));
        assert!(!net1.overlaps(Nid::new([68, 0, 0, 0].into(), Ipv4Mask::from_bitcount(8))));
        assert!(net1.overlaps(Nid::new([66, 2, 4, 5].into(), Ipv4Mask::from_bitcount(4))));
    }
}
