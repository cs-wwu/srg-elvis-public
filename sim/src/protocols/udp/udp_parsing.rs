use super::udp_misc::UdpError;
use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};

const HEADER_OCTETS: u16 = 8;

pub(super) struct UdpHeader {
    pub source: u16,
    pub destination: u16,
    // Todo: Consider removing unused header parts. For now, it's nice having
    // these available for the tests.
    #[allow(dead_code)]
    pub length: u16,
    #[allow(dead_code)]
    pub checksum: u16,
}

impl UdpHeader {
    pub fn from_bytes_ipv4(
        mut bytes: impl Iterator<Item = u8>,
        source_address: Ipv4Address,
        destination_address: Ipv4Address,
    ) -> Result<Self, UdpError> {
        let mut next = || -> Result<u8, UdpError> { bytes.next().ok_or(UdpError::HeaderTooShort) };

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

        let bytes_consumed = next_padded(&mut bytes, &mut checksum) + 8;

        if bytes_consumed != length || bytes.next().is_some() {
            Err(UdpError::LengthMismatch)?
        }

        let actual_checksum = checksum.as_u16();
        if actual_checksum != expected_checksum {
            Err(UdpError::InvalidChecksum {
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

pub(super) fn build_udp_header(
    source_address: Ipv4Address,
    source_port: u16,
    destination_address: Ipv4Address,
    destination_port: u16,
    mut payload: impl Iterator<Item = u8>,
) -> Result<Vec<u8>, UdpError> {
    let mut checksum = Checksum::new();
    let length = next_padded(&mut payload, &mut checksum);

    let length = HEADER_OCTETS
        .checked_add(length.try_into().map_err(|_| UdpError::OverlyLongPayload)?)
        .ok_or(UdpError::OverlyLongPayload)?;

    // Once for the header, again for the pseudo header
    checksum.add_u16(length);
    checksum.add_u16(length);

    checksum.add_u32(source_address.into());
    checksum.add_u32(destination_address.into());
    checksum.add_u8(0, 17);
    checksum.add_u16(source_port);
    checksum.add_u16(destination_port);

    let mut out = vec![];
    out.extend_from_slice(&source_port.to_be_bytes());
    out.extend_from_slice(&destination_port.to_be_bytes());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&checksum.as_u16().to_be_bytes());
    Ok(out)
}

fn next_padded(payload: &mut impl Iterator<Item = u8>, checksum: &mut Checksum) -> u16 {
    let mut length = 0;
    while let Some(first) = payload.next() {
        let second = match payload.next() {
            Some(second) => {
                length += 2;
                second
            }
            None => {
                length += 1;
                0
            }
        };
        checksum.add_u8(first, second);
    }
    length
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
