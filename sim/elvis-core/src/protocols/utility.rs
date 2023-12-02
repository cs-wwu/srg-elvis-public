//! Contains utilities for implementing protocols.

use super::ipv4::Ipv4Address;

/// A calculator for the checksum used by the UDP, TCP, and IP protocols.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum(u16);

impl Checksum {
    /// Creates a new checksum calculator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a `u16` to the checksum value.
    #[cfg(feature = "compute_checksum")]
    pub fn add_u16(&mut self, value: u16) {
        let (sum, carry) = self.0.overflowing_add(value);
        self.0 = sum + carry as u16;
    }

    #[cfg(not(feature = "compute_checksum"))]
    pub fn add_u16(&mut self, _value: u16) {}

    /// Adds `u16` formed by two `u8`s to the checksum value.
    pub fn add_u8(&mut self, a: u8, b: u8) {
        self.add_u16(u16::from_be_bytes([a, b]));
    }

    /// Adds two `u16`s to the checksum value by splitting a `u32` in half.
    pub fn add_u32(&mut self, value: [u8; 4]) {
        self.add_u8(value[0], value[1]);
        self.add_u8(value[2], value[3]);
    }

    /// Repeatedly gets the next two bytes at a `u16` from a byte iterator. If the `payload`
    /// contains an odd number of bytes, the last `u8` will be appended with the
    /// value zero.
    #[cfg(feature = "compute_checksum")]
    pub fn accumulate_remainder(&mut self, mut payload: impl Iterator<Item = u8>) {
        while let Some(a) = payload.next() {
            self.add_u8(a, payload.next().unwrap_or(0));
        }
    }

    /// Repeatedly gets the next two bytes at a `u16` from a byte iterator. If the `payload`
    /// contains an odd number of bytes, the last `u8` will be appended with the
    /// value zero.
    #[cfg(not(feature = "compute_checksum"))]
    pub fn accumulate_remainder(&mut self, _payload: impl Iterator<Item = u8>) {}

    /// Computes the final checksum value.
    #[cfg(feature = "compute_checksum")]
    pub fn as_u16(&self) -> u16 {
        match self.0 {
            // Use that there are two one's complement representations of zero
            // and pick the nonzero one to differentiate from an unused
            // checksum.
            0xffff => 0xffff,
            sum => !sum,
        }
    }

    /// Computes the final checksum value.
    #[cfg(not(feature = "compute_checksum"))]
    pub fn as_u16(&self) -> u16 {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Endpoint {
    pub address: Ipv4Address,
    pub port: u16,
}

impl Endpoint {
    pub const fn new(address: Ipv4Address, port: u16) -> Self {
        Self { address, port }
    }

    pub fn new_vec(addresses: &[Ipv4Address], port: u16) -> Vec<Endpoint> {
        let mut endpoints : Vec<Endpoint> = Vec::new();
        addresses
            .iter()
            .for_each(|ip| 
                endpoints.push(Endpoint::new(*ip, port)));
        endpoints
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Endpoints {
    pub local: Endpoint,
    pub remote: Endpoint,
}

impl Endpoints {
    pub const fn new(local: Endpoint, remote: Endpoint) -> Self {
        Self { local, remote }
    }

    pub const fn reverse(self) -> Self {
        Self {
            local: self.remote,
            remote: self.local,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortPair {
    pub local: u16,
    pub remote: u16,
}

impl PortPair {
    #[allow(unused)]
    pub const fn new(local: u16, remote: u16) -> Self {
        Self { local, remote }
    }

    #[allow(unused)]
    pub const fn reverse(self) -> Self {
        Self {
            local: self.remote,
            remote: self.local,
        }
    }
}

/// An extension trait for Iterator<Item = u8>. This should make it easier to parse bytes.
/// This adds methods for reading numbers from the iterator, such as u64s.
///
/// # Example
///
/// ```ignore
/// # use elvis_core::protocols::utility::BytesExt;
/// let arr: [u8] = [0xFF, 0x01, 0x09, 0x69];
/// let iter = arr.iter();
/// assert_eq!(iter.next_u16_be(), Some(0xFF01)); // 0xFF01 is 65281
/// assert_eq!(iter.next_u8(), Some(0x09));
/// assert_eq!(iter.next_u64_be(), None); // There are not enough bytes to make a u64
/// ```
pub trait BytesExt: Iterator<Item = u8> {
    /// Advances the iterator and returns the next value.
    /// Functions identically to `Iterator<Item = u8>::next`.
    fn next_u8(&mut self) -> Option<u8> {
        self.next()
    }

    /// Advances the iterator by 2 bytes.
    /// Combines these 2 bytes in big-endian order into a u16.
    /// Returns None if there were fewer than 2 bytes left in the iterator.
    fn next_u16_be(&mut self) -> Option<u16> {
        let arr = [self.next()?, self.next()?];
        Some(u16::from_be_bytes(arr))
    }

    /// Advances the iterator by 4 bytes.
    /// Combines these 4 bytes in big-endian order into a u32.
    /// Returns None if there were fewer than 4 bytes left in the iterator.
    fn next_u32_be(&mut self) -> Option<u32> {
        let arr = [self.next()?, self.next()?, self.next()?, self.next()?];
        Some(u32::from_be_bytes(arr))
    }

    /// Advances the iterator by 6 bytes.
    /// Combines these 6 bytes in big-endian order into a u64. The first 2 bytes of this number will be 0,
    /// the next 6 will be the ones that were read from the iterator.
    /// Returns None if there were fewer than 6 bytes left in the iterator.
    fn next_u48_be(&mut self) -> Option<u64> {
        let arr = [
            0,
            0,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
        ];
        Some(u64::from_be_bytes(arr))
    }

    /// Advances the iterator by 8 bytes.
    /// Combines these 8 bytes in big-endian order into a u64.
    /// Returns None if there were fewer than 8 bytes left in the iterator.
    fn next_u64_be(&mut self) -> Option<u64> {
        let arr = [
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
        ];
        Some(u64::from_be_bytes(arr))
    }

    /// Advances the iterator by 4 bytes.
    /// Combines these bytes in big-endian order into an [`Ipv4Address`].
    /// Returns None if there were fewer than 4 bytes left in the iterator.
    fn next_ipv4addr(&mut self) -> Option<Ipv4Address> {
        self.next_u32_be().map(Ipv4Address::from)
    }

    /// Collects the next `N` items of the iterator into an array.
    /// Returns `None` if there were fewer than `N` bytes left in the iterator.
    /// (This actually part of the rust std ([`Iterator::next_chunk`]), 
    /// but it's nightly only at the moment of writing.)
    fn next_n<const N: usize>(&mut self) -> Option<[u8; N]> {
        // I was tempted to use unsafe code so the array doesn't get initialized
        // Not today, Satan
        let mut result = [0; N];
        for element in &mut result {
            *element = self.next()?
        }
        Some(result)
    }
}

impl<T: Iterator<Item = u8>> BytesExt for T {
    // yippee
}

/// The doc tests don't work because utility is a private module!
/// so I'm putting this here
#[cfg(test)]
mod tests {
    use crate::protocols::utility::BytesExt;
    #[test]
    fn test_bytes_ext() {
        let arr = [0xFF, 0x01, 0x09, 0x69];
        // cloned is necessary so that we can iterate over u8 instead of &u8
        let mut iter = arr.iter().cloned();
        assert_eq!(iter.next_u16_be(), Some(0xFF01)); // 0xFF01 is 65281
        assert_eq!(iter.next_u8(), Some(0x09));
        assert_eq!(iter.next_u64_be(), None); // There are not enough bytes to make a u64
    }

    #[test]
    fn bytes_ext_u48() {
        let arr = [
            0x00, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78,
        ];
        let mut iter = arr.iter().cloned();
        assert_eq!(iter.next_u8(), Some(0x00));
        assert_eq!(iter.next_u48_be(), Some(0x123456781234));
        assert_eq!(iter.next_u48_be(), Some(0x567812345678));
        assert_eq!(iter.next_u48_be(), None);

        let arr2 = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut iter = arr2.iter().cloned();
        assert_eq!(iter.next_u48_be(), None);
    }
}
