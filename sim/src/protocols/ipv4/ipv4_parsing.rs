use super::{ipv4_misc::Ipv4Error, Ipv4Address};
use crate::protocols::utility::Checksum;

// Note: There are many #[allow(dead_code)] flags in this file. None of this
// stuff is public and not all of it is being used internally, but we want to
// have the APIs built out for future use.

const BASE_WORDS: u8 = 5;
const BASE_OCTETS: u16 = BASE_WORDS as u16 * 4;
const FRAGMENT_OFFSET_MASK: u16 = 0x1fff;

/// An IPv4 header, as described in RFC791 p11 s3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Ipv4Header {
    pub ihl: u8,
    pub type_of_service: TypeOfService,
    pub total_length: u16,
    pub identification: u16,
    pub fragment_offset: u16,
    pub flags: ControlFlags,
    pub time_to_live: u8,
    pub protocol: u8,
    // Todo: This isn't needed after parsing in main line code, but it is nice
    // for testing and for completeness. Consider whether it is worth removing.
    pub checksum: u16,
    pub source: Ipv4Address,
    pub destination: Ipv4Address,
}

impl Ipv4Header {
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, Ipv4Error> {
        let mut next =
            || -> Result<u8, Ipv4Error> { bytes.next().ok_or(Ipv4Error::HeaderTooShort) };

        let mut checksum = Checksum::new();

        let version_and_ihl = next()?;
        let version = version_and_ihl >> 4;
        if version != 4 {
            Err(Ipv4Error::IncorrectIpv4Version)?
        }
        let ihl = version_and_ihl & 0b1111;
        if ihl != BASE_WORDS {
            // Todo: Support optional headers
            Err(Ipv4Error::InvalidHeaderLength)?
        }
        let type_of_service_byte = next()?;
        let reserved = type_of_service_byte & 0b11;
        if reserved != 0 {
            Err(Ipv4Error::UsedReservedTos)?
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
            Err(Ipv4Error::UsedReservedFlag)?
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
            Err(Ipv4Error::IncorrectChecksum {
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
    pub fn new(
        source: Ipv4Address,
        destination: Ipv4Address,
        protocol: ProtocolNumber,
        payload_length: u16,
    ) -> Self {
        Self {
            type_of_service: Default::default(),
            payload_length,
            identification: 0,
            fragment_offset: 0,
            flags: Default::default(),
            time_to_live: 30,
            protocol: protocol as u8,
            source,
            destination,
        }
    }

    pub fn type_of_service(mut self, type_of_service: TypeOfService) -> Self {
        self.type_of_service = type_of_service;
        self
    }

    pub fn identification(mut self, identification: u16) -> Self {
        self.identification = identification;
        self
    }

    pub fn fragment_offset(mut self, fragment_offset: u16) -> Self {
        self.fragment_offset = fragment_offset;
        self
    }

    pub fn flags(mut self, flags: ControlFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn build(self) -> Result<Vec<u8>, Ipv4Error> {
        let mut checksum = Checksum::new();

        let version_and_ihl = (4u8 << 4) | BASE_WORDS;
        let type_of_service = self.type_of_service.as_u8();
        checksum.add_u8(version_and_ihl, type_of_service);

        let total_length = self
            .payload_length
            .checked_add(BASE_OCTETS)
            .ok_or(Ipv4Error::OverlyLongPayload)?;
        checksum.add_u16(total_length);

        checksum.add_u16(self.identification);

        if self.fragment_offset > FRAGMENT_OFFSET_MASK {
            Err(Ipv4Error::OverlyLongFragmentOffset)?
        }
        let flags_and_fragment_offset =
            ((self.flags.as_u8() as u16) << 13) | (self.fragment_offset & FRAGMENT_OFFSET_MASK);
        checksum.add_u16(flags_and_fragment_offset);

        checksum.add_u8(self.time_to_live, self.protocol as u8);
        checksum.add_u32(self.source.into());
        checksum.add_u32(self.destination.into());

        let mut out = vec![];
        out.push(version_and_ihl);
        out.push(type_of_service);
        out.extend_from_slice(&total_length.to_be_bytes());
        out.extend_from_slice(&self.identification.to_be_bytes());
        out.extend_from_slice(&flags_and_fragment_offset.to_be_bytes());
        out.push(self.time_to_live);
        out.push(self.protocol as u8);
        out.extend_from_slice(&checksum.as_u16().to_be_bytes());
        out.extend_from_slice(&self.source.to_u32().to_be_bytes());
        out.extend_from_slice(&self.destination.to_u32().to_be_bytes());
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum ProtocolNumber {
    // Todo: Expand this list as we support more protocols out of the box.
    // https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml
    Icpm = 1,
    Igmp = 2,
    Ipv4 = 4,
    Tcp = 6,
    Udp = 17,
    Ipv6 = 41,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) struct ControlFlags(u8);

impl ControlFlags {
    pub fn new(may_fragment: bool, is_last_fragment: bool) -> Self {
        Self((!is_last_fragment as u8) | ((!may_fragment as u8) << 1))
    }

    #[allow(dead_code)]
    pub fn may_fragment(&self) -> bool {
        self.0 & 0b10 == 0
    }

    #[allow(dead_code)]
    pub fn is_last_fragment(&self) -> bool {
        self.0 & 0b1 == 0
    }

    pub fn as_u8(self) -> u8 {
        self.into()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) struct TypeOfService(u8);

impl TypeOfService {
    // Note: It should not be possible for any of these functions to fail
    // because the enum variants cover any possible byte value we would be
    // passing in.

    pub fn new(
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
pub(super) enum Delay {
    Normal = 0,
    Low = 1,
}

impl TryFrom<u8> for Delay {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Low),
            _ => Err(Ipv4Error::Delay(byte)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Throughput {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Throughput {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(Ipv4Error::Throughput(byte)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Reliability {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Reliability {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(Ipv4Error::Reliability(byte)),
        }
    }
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
pub(super) enum Precedence {
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
    type Error = Ipv4Error;

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
            _ => Err(Ipv4Error::Precedence(byte)),
        }
    }
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
        let serial_header = {
            let mut serial_header = vec![];
            header.write(&mut serial_header).unwrap();
            serial_header
        };
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
            ProtocolNumber::Udp,
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
