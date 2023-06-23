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

use std::{net::Ipv4Addr, str::FromStr};

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
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    /// Turns the mask into a u32.
    pub fn to_u32(self) -> u32 {
        self.into()
    }

    /// Turns the mask into an Ipv4 address.
    pub fn to_ipv4_address(self) -> Ipv4Address {
        self.into()
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

/// Turns an string in [CIDR notation](https://en.wikipedia.org/wiki/Classless_Inter-Domain_Routing#CIDR_notation)
/// into an Ipv4 address and a subnet mask.
///
/// Returns an error if the string is not of form `ip_address/mask_length`.
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

#[derive(Clone, Debug, thiserror::Error)]
#[error("Failed to parse CIDR string")]
pub enum CidrParseError {
    Ipv4,
    Mask(#[from] std::num::ParseIntError),
}

/// Returns the number of IP addresses in a network based off the mask.
///
/// # Examples
///
/// ```
/// # use elvis_core::protocols::arp::subnetting::*;
/// // A 4-bit host portion allows for 2^4 or 16 IP addresses
/// // including network ID and broadcast IP
/// let ips = ips_in_net(Ipv4Mask::from_bitcount(28));
/// assert_eq!(ips, 16);
/// ```
pub fn ips_in_net(mask: Ipv4Mask) -> u64 {
    let wildcard = !(mask.to_u32());
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
/// let ips = usable_ips(Ipv4Mask::from_bitcount(28));
/// assert_eq!(ips, 14);
/// ```
pub fn usable_ips(mask: Ipv4Mask) -> u32 {
    let wildcard = !(mask.to_u32());
    match wildcard {
        0 | 1 => 0,
        other => other - 1,
    }
}

/// Returns the network ID based on an IP address and subnet mask.
///
/// # Example
///
/// ```
/// # use elvis_core::protocols::arp::subnetting::*;
/// # use elvis_core::protocols::ipv4::*;
/// let (ip, mask) = cidr_to_ip("10.0.0.119/24").unwrap();
/// let broadcast = get_network_id(ip, mask);
/// assert_eq!(broadcast, Ipv4Address::from([10,0,0,0]));
/// ```
pub fn get_network_id(ip: Ipv4Address, mask: Ipv4Mask) -> Ipv4Address {
    (ip.to_u32() & mask.to_u32()).into()
}

/// Returns the broadcast IP address for this network based on an IP address and subnet mask.
///
/// # Example
///
/// ```
/// # use elvis_core::protocols::arp::subnetting::*;
/// # use elvis_core::protocols::ipv4::*;
/// let (ip, mask) = cidr_to_ip("10.0.0.119/24").unwrap();
/// let broadcast = get_broadcast_address(ip, mask);
/// assert_eq!(broadcast, Ipv4Address::from([10,0,0,255]));
/// ```
pub fn get_broadcast_address(ip: Ipv4Address, mask: Ipv4Mask) -> Ipv4Address {
    let ip_id = get_network_id(ip, mask);
    let new_ip_u32 = ip_id.to_u32() + (!mask.to_u32());
    new_ip_u32.into()
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
