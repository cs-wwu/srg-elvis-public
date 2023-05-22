use super::Ipv4Address;
use crate::protocols::utility::Checksum;
use std::fmt::{self, Debug, Formatter};
use thiserror::Error as ThisError;

// Note: There are many #[allow(dead_code)] flags in this file. None of this
// stuff is public and not all of it is being used internally, but we want to
// have the APIs built out for future use.

/// The number of `u32` words in a basic IPv4 header
const BASE_WORDS: u8 = 5;
/// The number of `u8` bytes in a basic IPv4 header
const BASE_OCTETS: u16 = BASE_WORDS as u16 * 4;
/// This is bitwise anded with the `u16` containing flags and fragment offset to
/// extract the fragment offset part.
const FRAGMENT_OFFSET_MASK: u16 = 0x1fff;

/// An IPv4 header, as described in RFC791 p11 s3.1
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ipv4Header {
    /// Internet Header Length, the number of `u32` words in the IPv4 header
    pub ihl: u8,
    /// The quality of service desired
    pub type_of_service: TypeOfService,
    /// The length of the datagram in bytes
    pub total_length: u16,
    /// Assigned by the sender to aid in assembling fragments
    pub identification: u16,
    /// Where in the datagram this fragment belongs in units of 8 bytes
    pub fragment_offset: u16,
    /// Flags describing fragmentation properties
    pub flags: ControlFlags,
    /// The number of remaining hops this datagram can take before being removed
    pub time_to_live: u8,
    /// Indicates the next level protocol in the data portion of the datagram
    pub protocol: u8,
    /// The IPv4 header checksum
    pub checksum: u16,
    /// The source address
    pub source: Ipv4Address,
    /// The destination address
    pub destination: Ipv4Address,
}

impl Ipv4Header {
    /// Parses a header from a byte iterator.
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };

        let mut checksum = Checksum::new();

        let version_and_ihl = next()?;
        let version = version_and_ihl >> 4;
        if version != 4 {
            Err(ParseError::IncorrectIpv4Version)?
        }
        let ihl = version_and_ihl & 0b1111;
        if ihl != BASE_WORDS {
            // TODO(hardint): Support optional headers
            Err(ParseError::InvalidHeaderLength)?
        }
        let type_of_service_byte = next()?;
        let reserved = type_of_service_byte & 0b11;
        if reserved != 0 {
            Err(ParseError::UsedReservedTos)?
        }
        checksum.add_u8(version_and_ihl, type_of_service_byte);

        let total_length = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(total_length);

        let identification = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(identification);

        let flags_and_fragment_offset_bytes = u16::from_be_bytes([next()?, next()?]);
        let fragment_offset = flags_and_fragment_offset_bytes & FRAGMENT_OFFSET_MASK;
        let control_flag_bits = (flags_and_fragment_offset_bytes >> 13) as u8;
        if control_flag_bits & 0b100 != 0 {
            Err(ParseError::UsedReservedFlag)?
        }
        checksum.add_u16(flags_and_fragment_offset_bytes);

        let time_to_live = next()?;
        let protocol = next()?;
        checksum.add_u8(time_to_live, protocol);

        let expected_checksum = u16::from_be_bytes([next()?, next()?]);

        let source_bytes = [next()?, next()?, next()?, next()?];
        let source: Ipv4Address = u32::from_be_bytes(source_bytes).into();
        checksum.add_u32(source_bytes);

        let destination_bytes = [next()?, next()?, next()?, next()?];
        let destination: Ipv4Address = u32::from_be_bytes(destination_bytes).into();
        checksum.add_u32(destination_bytes);

        let actual_checksum = checksum.as_u16();
        if actual_checksum != expected_checksum {
            Err(ParseError::Checksum {
                expected: expected_checksum,
                actual: actual_checksum,
            })?
        }

        Ok(Self {
            ihl,
            type_of_service: type_of_service_byte.into(),
            total_length,
            identification,
            fragment_offset,
            flags: control_flag_bits.into(),
            time_to_live,
            protocol,
            checksum: expected_checksum,
            source,
            destination,
        })
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The IPv4 header is incomplete")]
    HeaderTooShort,
    #[error("{0}")]
    Reliability(#[from] ReliabilityError),
    #[error("{0}")]
    Delay(#[from] DelayError),
    #[error("{0}")]
    Throughput(#[from] ThroughputError),
    #[error("{0}")]
    Precedence(#[from] PrecedenceError),
    #[error("The reserved bits in type of service are nonzero")]
    UsedReservedTos,
    #[error("Expected version 4 in IPv4 header")]
    IncorrectIpv4Version,
    #[error("The reserved control flags bit was used")]
    UsedReservedFlag,
    #[error("Expected 5 bytes for IPv4 header")]
    InvalidHeaderLength,
    #[error(
        "The header checksum {expected:#06x} does not match the calculated checksum {actual:#06x}"
    )]
    Checksum { expected: u16, actual: u16 },
}

/// A builder for IPv4 headers. The fields align with those found on [`Ipv4Header`].
pub(super) struct Ipv4HeaderBuilder {
    type_of_service: TypeOfService,
    payload_length: u16,
    identification: u16,
    fragment_offset: u16,
    flags: ControlFlags,
    time_to_live: u8,
    protocol: u8,
    source: Ipv4Address,
    destination: Ipv4Address,
}

impl Ipv4HeaderBuilder {
    /// Creates a new builder.
    pub fn new(
        source: Ipv4Address,
        destination: Ipv4Address,
        protocol: u8,
        payload_length: u16,
    ) -> Self {
        Self {
            type_of_service: Default::default(),
            payload_length,
            identification: 0,
            fragment_offset: 0,
            flags: Default::default(),
            time_to_live: 30,
            protocol,
            source,
            destination,
        }
    }

    /// Sets the type of service
    #[allow(dead_code)]
    pub fn type_of_service(mut self, type_of_service: TypeOfService) -> Self {
        self.type_of_service = type_of_service;
        self
    }

    /// Sets the identification field
    #[allow(dead_code)]
    pub fn identification(mut self, identification: u16) -> Self {
        self.identification = identification;
        self
    }

    /// Sets the fragment offset field
    #[allow(dead_code)]
    pub fn fragment_offset(mut self, fragment_offset: u16) -> Self {
        // TODO(hardint): Check that `fragment_offset` fits within the fragment
        // offset mask
        self.fragment_offset = fragment_offset;
        self
    }

    /// Sets the control flags
    #[allow(dead_code)]
    pub fn flags(mut self, flags: ControlFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Creates a serialized header from the configuration provided
    pub fn build(self) -> Result<Vec<u8>, HeaderBuildError> {
        let mut checksum = Checksum::new();

        let version_and_ihl = (4u8 << 4) | BASE_WORDS;
        let type_of_service = self.type_of_service.as_u8();
        checksum.add_u8(version_and_ihl, type_of_service);

        let total_length = self
            .payload_length
            .checked_add(BASE_OCTETS)
            .ok_or(HeaderBuildError::OverlyLongPayload)?;
        checksum.add_u16(total_length);

        checksum.add_u16(self.identification);

        if self.fragment_offset > FRAGMENT_OFFSET_MASK {
            Err(HeaderBuildError::OverlyLongFragmentOffset)?
        }
        let flags_and_fragment_offset =
            ((self.flags.as_u8() as u16) << 13) | (self.fragment_offset & FRAGMENT_OFFSET_MASK);
        checksum.add_u16(flags_and_fragment_offset);

        checksum.add_u8(self.time_to_live, self.protocol);
        checksum.add_u32(self.source.into());
        checksum.add_u32(self.destination.into());

        let mut out = Vec::with_capacity(BASE_OCTETS as usize);
        out.push(version_and_ihl);
        out.push(type_of_service);
        out.extend_from_slice(&total_length.to_be_bytes());
        out.extend_from_slice(&self.identification.to_be_bytes());
        out.extend_from_slice(&flags_and_fragment_offset.to_be_bytes());
        out.push(self.time_to_live);
        out.push(self.protocol);
        out.extend_from_slice(&checksum.as_u16().to_be_bytes());
        out.extend_from_slice(&self.source.to_u32().to_be_bytes());
        out.extend_from_slice(&self.destination.to_u32().to_be_bytes());
        Ok(out)
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum HeaderBuildError {
    #[error("The payload is longer than is allowed")]
    OverlyLongPayload,
    #[error("The fragment offset is too long to fit control flags in the header")]
    OverlyLongFragmentOffset,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlFlags(u8);

impl ControlFlags {
    pub const DEFAULT: Self = Self::new(true, true);

    #[allow(dead_code)]
    pub const fn new(may_fragment: bool, is_last_fragment: bool) -> Self {
        Self((!is_last_fragment as u8) | ((!may_fragment as u8) << 1))
    }

    pub const fn may_fragment(&self) -> bool {
        self.0 & 0b10 == 0
    }

    pub fn set_may_fragment(&mut self, value: bool) {
        self.0 = (self.0 & 0b01) | ((!value as u8) << 1);
    }

    pub const fn is_last_fragment(&self) -> bool {
        self.0 & 0b01 == 0
    }

    pub fn set_is_last_fragment(&mut self, value: bool) {
        self.0 = (self.0 & 0b10) | !value as u8;
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl Debug for ControlFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlFlags")
            .field("MayFrag", &self.may_fragment())
            .field("LastFrag", &self.is_last_fragment())
            .finish()
    }
}

impl Default for ControlFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<u8> for ControlFlags {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}

impl From<ControlFlags> for u8 {
    fn from(flags: ControlFlags) -> Self {
        flags.0
    }
}

/// The Type of Service provides an indication of the abstract
/// parameters of the quality of service desired.  These parameters are
/// to be used to guide the selection of the actual service parameters
/// when transmitting a datagram through a particular network.  Several
/// networks offer service precedence, which somehow treats high
/// precedence traffic as more important than other traffic (generally
/// by accepting only traffic above a certain precedence at time of high
/// load).  The major choice is a three way tradeoff between low-delay,
/// high-reliability, and high-throughput.
///
/// See RFC791 p11 s3.1 for more details.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeOfService(u8);

impl TypeOfService {
    // Note: It should not be possible for any of these functions to fail
    // because the enum variants cover any possible byte value we would be
    // passing in.

    pub const DEFAULT: Self = Self::new(
        Precedence::Routine,
        Delay::Normal,
        Throughput::Normal,
        Reliability::Normal,
    );

    #[allow(dead_code)]
    pub const fn new(
        precedence: Precedence,
        delay: Delay,
        throughput: Throughput,
        reliability: Reliability,
    ) -> Self {
        Self(
            ((reliability as u8) << 2)
                | ((throughput as u8) << 3)
                | ((delay as u8) << 4)
                | ((precedence as u8) << 5),
        )
    }

    #[allow(dead_code)]
    pub fn precedence(&self) -> Precedence {
        (self.0 >> 5).try_into().unwrap()
    }

    #[allow(dead_code)]
    pub fn delay(&self) -> Delay {
        ((self.0 >> 4) & 0b1).try_into().unwrap()
    }

    #[allow(dead_code)]
    pub fn throughput(&self) -> Throughput {
        ((self.0 >> 3) & 0b1).try_into().unwrap()
    }

    #[allow(dead_code)]
    pub fn reliability(&self) -> Reliability {
        ((self.0 >> 2) & 0b1).try_into().unwrap()
    }

    pub fn as_u8(self) -> u8 {
        self.into()
    }
}

impl Debug for TypeOfService {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeOfService")
            .field("precedence", &self.precedence())
            .field("delay", &self.delay())
            .field("throughput", &self.throughput())
            .field("reliability", &self.reliability())
            .finish()
    }
}

impl From<u8> for TypeOfService {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}

impl From<TypeOfService> for u8 {
    fn from(type_of_service: TypeOfService) -> Self {
        type_of_service.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Delay {
    Normal = 0,
    Low = 1,
}

impl TryFrom<u8> for Delay {
    type Error = DelayError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Low),
            _ => Err(DelayError::Conversion(byte)),
        }
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum DelayError {
    #[error("Could not convert from {0}")]
    Conversion(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Throughput {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Throughput {
    type Error = ThroughputError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(ThroughputError::Conversion(byte)),
        }
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ThroughputError {
    #[error("Could not convert from {0}")]
    Conversion(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Reliability {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Reliability {
    type Error = ReliabilityError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(ReliabilityError::Conversion(byte)),
        }
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ReliabilityError {
    #[error("Could not convert from {0}")]
    Conversion(u8),
}

/// The Network Control precedence designation is intended to be used within a
/// network only.  The actual use and control of that designation is up to each
/// network. The Internetwork Control designation is intended for use by gateway
/// control originators only. If the actual use of these precedence designations
/// is of concern to a particular network, it is the responsibility of that
/// network to control the access to, and use of, those precedence designations.
///
/// Described in RFC791 p13 s3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Precedence {
    NetworkControl = 0b111,
    InternetworkControl = 0b110,
    CriticEcp = 0b101,
    FlashOverride = 0b100,
    Flash = 0b011,
    Immediate = 0b010,
    Priority = 0b001,
    Routine = 0b000,
}

impl TryFrom<u8> for Precedence {
    type Error = PrecedenceError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0b111 => Ok(Self::NetworkControl),
            0b110 => Ok(Self::InternetworkControl),
            0b101 => Ok(Self::CriticEcp),
            0b100 => Ok(Self::FlashOverride),
            0b011 => Ok(Self::Flash),
            0b010 => Ok(Self::Immediate),
            0b001 => Ok(Self::Priority),
            0b000 => Ok(Self::Routine),
            _ => Err(PrecedenceError::Conversion(byte)),
        }
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum PrecedenceError {
    #[error("Could not convert from {0}")]
    Conversion(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header() -> (etherparse::Ipv4Header, Vec<u8>, u16) {
        let payload = "Hello, world!";
        let ttl = 30;
        let protocol = etherparse::IpNumber::Udp;
        let source = [127, 0, 0, 1];
        let destination = [123, 45, 67, 89];
        let header = etherparse::Ipv4Header::new(
            payload.len().try_into().unwrap(),
            ttl,
            protocol,
            source,
            destination,
        );
        #[allow(unused_mut)]
        let mut serial_header = {
            let mut serial_header = vec![];
            header.write(&mut serial_header).unwrap();
            serial_header
        };

        #[cfg(not(feature = "compute_checksum"))]
        {
            serial_header[10] = 0;
            serial_header[11] = 0;
        }

        (header, serial_header, payload.len().try_into().unwrap())
    }

    #[test]
    fn parses_basic_header() -> anyhow::Result<()> {
        let (valid_header, serial_header, _) = make_header();
        let parsed = Ipv4Header::from_bytes(serial_header.iter().cloned())?;
        assert_eq!(parsed.ihl, valid_header.ihl());
        assert_eq!(parsed.type_of_service.delay(), Delay::Normal);
        assert_eq!(parsed.type_of_service.throughput(), Throughput::Normal);
        assert_eq!(parsed.type_of_service.reliability(), Reliability::Normal);
        assert_eq!(parsed.type_of_service.precedence(), Precedence::Routine);
        assert_eq!(parsed.total_length, valid_header.total_len());
        assert_eq!(parsed.identification, valid_header.identification);
        assert_eq!(
            parsed.flags.is_last_fragment(),
            !valid_header.more_fragments
        );
        assert_eq!(
            parsed.flags.may_fragment(),
            valid_header.is_fragmenting_payload()
        );
        assert_eq!(parsed.fragment_offset, 0);
        assert_eq!(parsed.time_to_live, valid_header.time_to_live);
        assert_eq!(parsed.protocol, valid_header.protocol);
        #[cfg(feature = "compute_checksum")]
        assert_eq!(parsed.checksum, valid_header.calc_header_checksum()?);
        assert_eq!(parsed.source.to_bytes(), valid_header.source);
        assert_eq!(parsed.destination.to_bytes(), valid_header.destination);
        Ok(())
    }

    #[test]
    fn generates_basic_header() -> anyhow::Result<()> {
        let (_, expected, payload_length) = make_header();
        let actual = Ipv4HeaderBuilder::new(
            Ipv4Address::new([127, 0, 0, 1]),
            Ipv4Address::new([123, 45, 67, 89]),
            17,
            payload_length,
        )
        .flags(ControlFlags::new(false, true))
        .type_of_service(TypeOfService::new(
            Precedence::Routine,
            Delay::Normal,
            Throughput::Normal,
            Reliability::Normal,
        ))
        .build()?;
        assert_eq!(actual, expected);
        Ok(())
    }
}
