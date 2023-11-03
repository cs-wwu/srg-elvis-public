use crate::protocols::{
    ipv4::Ipv4Address,
    utility::{BytesExt, Checksum},
};
use thiserror::Error as ThisError;

/// The number of bytes in a UDP header
const HEADER_OCTETS: u16 = 8;

/// Represents a UDP header, either one that was parsed or one we are going to
/// serialize
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpHeader {
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
        mut packet: impl Iterator<Item = u8>,
        packet_len: usize,
        source_address: Ipv4Address,
        destination_address: Ipv4Address,
    ) -> Result<Self, ParseError> {
        const HTS: ParseError = ParseError::HeaderTooShort;

        let mut checksum = Checksum::new();

        let source_port = packet.next_u16_be().ok_or(HTS)?;
        checksum.add_u16(source_port);

        let destination_port = packet.next_u16_be().ok_or(HTS)?;
        checksum.add_u16(destination_port);

        let length = packet.next_u16_be().ok_or(HTS)?;
        checksum.add_u16(length);
        // This is used a second time in the pseudo header
        checksum.add_u16(length);

        let expected_checksum = packet.next_u16_be().ok_or(HTS)?;

        // Pseudo header parts
        checksum.add_u32(source_address.into());
        checksum.add_u32(destination_address.into());

        // [zero, UDP protocol number] from pseudo header
        checksum.add_u8(0, 17);

        checksum.accumulate_remainder(&mut packet);

        if packet_len != length as usize {
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
pub enum ParseError {
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
pub fn build_udp_header(
    source_address: Ipv4Address,
    source_port: u16,
    destination_address: Ipv4Address,
    destination_port: u16,
    mut text: impl Iterator<Item = u8>,
    text_len: usize,
) -> Result<Vec<u8>, BuildHeaderError> {
    let mut checksum = Checksum::new();
    checksum.accumulate_remainder(&mut text);

    let length: u16 = (text_len + HEADER_OCTETS as usize)
        .try_into()
        .map_err(|_| BuildHeaderError::OverlyLongPayload)?;

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
pub enum BuildHeaderError {
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

    fn etherparse_headers() -> (etherparse::UdpHeader, Vec<u8>, &'static str) {
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

        let udp_header = if cfg!(feature = "compute_checksum") {
            etherparse::UdpHeader::with_ipv4_checksum(
                SOURCE_PORT,
                DESTINATION_PORT,
                &ip_header,
                payload.as_bytes(),
            )
        } else {
            etherparse::UdpHeader::without_ipv4_checksum(
                SOURCE_PORT,
                DESTINATION_PORT,
                payload.len(),
            )
        }
        .unwrap();

        let serial = {
            let mut serial = vec![];
            udp_header.write(&mut serial).unwrap();
            serial
        };
        (udp_header, serial, payload)
    }

    #[test]
    fn parses_header() -> anyhow::Result<()> {
        let (expected, expected_serial, payload) = etherparse_headers();
        let len = expected_serial.len() + payload.len();
        let actual = UdpHeader::from_bytes_ipv4(
            expected_serial
                .into_iter()
                .chain(payload.as_bytes().iter().cloned()),
            len,
            SOURCE_ADDRESS.into(),
            DESTINATION_ADDRESS.into(),
        )?;
        assert_eq!(actual.source, expected.source_port);
        assert_eq!(actual.destination, expected.destination_port);
        assert_eq!(actual.length, expected.length);
        assert_eq!(actual.checksum, expected.checksum);
        Ok(())
    }

    #[test]
    fn generates_header() -> anyhow::Result<()> {
        let (_, expected, payload) = etherparse_headers();
        let actual = build_udp_header(
            SOURCE_ADDRESS.into(),
            SOURCE_PORT,
            DESTINATION_ADDRESS.into(),
            DESTINATION_PORT,
            payload.as_bytes().iter().cloned(),
            payload.len(),
        )?;
        assert_eq!(actual, expected);
        Ok(())
    }
}
