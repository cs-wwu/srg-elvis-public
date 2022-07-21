use super::udp_misc::UdpError;
use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};

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

        let mut bytes_consumed = 8;
        while bytes_consumed < length {
            let first = bytes.next().ok_or(UdpError::HeaderTooShort)?;
            // If we got an odd number of bytes, pad with a zero
            let second = match bytes.next() {
                Some(second) => {
                    bytes_consumed += 2;
                    second
                }
                None => {
                    bytes_consumed += 1;
                    0
                }
            };
            checksum.add_u16(u16::from_be_bytes([first, second]));
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_udp_header() -> anyhow::Result<()> {
        let payload = b"Hello, world!";
        let time_to_live = 30;
        let protocol = etherparse::IpNumber::Udp;
        let source_address = [127, 0, 0, 1];
        let destination_address = [123, 45, 67, 89];
        let ip_header = etherparse::Ipv4Header::new(
            payload.len().try_into()?,
            time_to_live,
            protocol,
            source_address,
            destination_address,
        );
        let source_port = 12345u16;
        let destination_port = 6789u16;
        let valid_header = etherparse::UdpHeader::with_ipv4_checksum(
            source_port,
            destination_port,
            &ip_header,
            payload,
        )?;
        let serial = {
            let mut serial = vec![];
            valid_header.write(&mut serial)?;
            serial
        };
        let actual = UdpHeader::from_bytes_ipv4(
            serial.into_iter().chain(payload.iter().cloned()),
            source_address.into(),
            destination_address.into(),
        )?;
        assert_eq!(actual.source, valid_header.source_port);
        assert_eq!(actual.destination, valid_header.destination_port);
        assert_eq!(actual.length, valid_header.length);
        assert_eq!(
            actual.checksum,
            valid_header
                .calc_checksum_ipv4(&ip_header, payload)
                .unwrap()
        );
        Ok(())
    }
}
