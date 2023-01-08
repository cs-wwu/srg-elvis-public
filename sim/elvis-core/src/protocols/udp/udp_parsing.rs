use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};
use thiserror::Error as ThisError;

/// The number of bytes in a UDP header
const HEADER_OCTETS: u16 = 8;

/// Represents a UDP header, either one that was parsed or one we are going to
/// serialize
pub(super) struct UdpHeader {
    /// The source port
    pub source: u16,
    /// The destination port
    pub destination: u16,
    // TODO(hardint): Consider removing unused header parts. For now, it's nice having
    // these available for the tests.
    /// The length of the UDP packet in bytes, including the header
    #[allow(dead_code)]
    pub length: u16,
    /// The UDP checksum
    #[allow(dead_code)]
    pub checksum: u16,
}

impl UdpHeader {
    /// Parses a UDP header from an iterator of bytes
    pub fn from_bytes_ipv4(
        mut bytes: impl Iterator<Item = u8>,
        source_address: Ipv4Address,
        destination_address: Ipv4Address,
    ) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };

        let mut checksum = Checksum::new();

        let source_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(source_port);

        let destination_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(destination_port);

        let length = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(length);
        // This is used a second time in the pseudo header
        checksum.add_u16(length);

        let expected_checksum = u16::from_be_bytes([next()?, next()?]);

        // Pseudo header parts
        checksum.add_u32(source_address.into());
        checksum.add_u32(destination_address.into());

        // [zero, UDP protocol number] from pseudo header
        checksum.add_u8(0, 17);

        let bytes_consumed = checksum.accumulate_remainder(&mut bytes) + 8;

        if bytes_consumed != length || bytes.next().is_some() {
            Err(ParseError::LengthMismatch)?
        }

        let actual_checksum = checksum.as_u16();
        if actual_checksum != expected_checksum {
            Err(ParseError::Checksum {
                actual: actual_checksum,
                expected: expected_checksum,
            })?
        }

        Ok(Self {
            source: source_port,
            destination: destination_port,
            length,
            checksum: expected_checksum,
        })
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseError {
    #[error("Too few bytes to constitute a UDP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    Checksum { actual: u16, expected: u16 },
    #[error("The number of message bytes differs from the header")]
    LengthMismatch,
}

/// Creates a serialized UDP packet header with the values provided
pub(super) fn build_udp_header(
    source_address: Ipv4Address,
    source_port: u16,
    destination_address: Ipv4Address,
    destination_port: u16,
    mut payload: impl Iterator<Item = u8>,
) -> Result<Vec<u8>, BuildHeaderError> {
    let mut checksum = Checksum::new();
    let length = checksum.accumulate_remainder(&mut payload);

    let length = HEADER_OCTETS
        .checked_add(length)
        .ok_or(BuildHeaderError::OverlyLongPayload)?;

    // Once for the header, again for the pseudo header
    checksum.add_u16(length);
    checksum.add_u16(length);

    checksum.add_u32(source_address.into());
    checksum.add_u32(destination_address.into());
    checksum.add_u8(0, 17);
    checksum.add_u16(source_port);
    checksum.add_u16(destination_port);

    let mut out = Vec::with_capacity(HEADER_OCTETS as usize);
    out.extend_from_slice(&source_port.to_be_bytes());
    out.extend_from_slice(&destination_port.to_be_bytes());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&checksum.as_u16().to_be_bytes());
    Ok(out)
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuildHeaderError {
    #[error("The UDP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_ADDRESS: [u8; 4] = [127, 0, 0, 1];
    const SOURCE_PORT: u16 = 12345;
    const DESTINATION_ADDRESS: [u8; 4] = [123, 45, 67, 89];
    const DESTINATION_PORT: u16 = 6789;

    fn etherparse_headers() -> (
        etherparse::Ipv4Header,
        etherparse::UdpHeader,
        Vec<u8>,
        &'static str,
    ) {
        let payload = "Hello, world!";
        let time_to_live = 30;
        let protocol = etherparse::IpNumber::Udp;
        let ip_header = etherparse::Ipv4Header::new(
            payload.len().try_into().unwrap(),
            time_to_live,
            protocol,
            SOURCE_ADDRESS,
            DESTINATION_ADDRESS,
        );
        let udp_header = etherparse::UdpHeader::with_ipv4_checksum(
            SOURCE_PORT,
            DESTINATION_PORT,
            &ip_header,
            payload.as_bytes(),
        )
        .unwrap();
        let serial = {
            let mut serial = vec![];
            udp_header.write(&mut serial).unwrap();
            serial
        };
        (ip_header, udp_header, serial, payload)
    }

    #[test]
    fn parses_header() -> anyhow::Result<()> {
        let (ip_header, expected, expected_serial, payload) = etherparse_headers();
        let actual = UdpHeader::from_bytes_ipv4(
            expected_serial
                .into_iter()
                .chain(payload.as_bytes().iter().cloned()),
            SOURCE_ADDRESS.into(),
            DESTINATION_ADDRESS.into(),
        )?;
        assert_eq!(actual.source, expected.source_port);
        assert_eq!(actual.destination, expected.destination_port);
        assert_eq!(actual.length, expected.length);
        assert_eq!(
            actual.checksum,
            expected
                .calc_checksum_ipv4(&ip_header, payload.as_bytes())
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn generates_header() -> anyhow::Result<()> {
        let (_, _, expected, payload) = etherparse_headers();
        let actual = build_udp_header(
            SOURCE_ADDRESS.into(),
            SOURCE_PORT,
            DESTINATION_ADDRESS.into(),
            DESTINATION_PORT,
            payload.as_bytes().iter().cloned(),
        )?;
        assert_eq!(actual, expected);
        Ok(())
    }
}
