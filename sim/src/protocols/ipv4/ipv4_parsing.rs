use super::ipv4_misc::Ipv4Error;

/// An IPv4 header, as described in RFC791 p11 s3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Ipv4Header {
    /// The protocol version
    // Todo: Remove this eventually
    pub version: u8,
    /// The internet header length
    pub ihl: u8,
}

impl Ipv4Header {
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, Ipv4Error> {
        let byte = bytes.next().ok_or(Ipv4Error::HeaderTooShort)?;
        let version = byte >> 4;
        let ihl = byte & 0b1111;
        Ok(Self { version, ihl })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_header() -> anyhow::Result<()> {
        let payload = "Hello, world!";
        let ttl = 30;
        let protocol = etherparse::IpNumber::Udp;
        let source = [127, 0, 0, 1];
        let destination = [123, 45, 67, 89];
        let mut valid_header = etherparse::Ipv4Header::new(
            payload.len().try_into()?,
            ttl,
            protocol,
            source,
            destination,
        );
        let mut serial_header = vec![];
        valid_header.write(&mut serial_header);
        let parsed = Ipv4Header::from_bytes(serial_header.iter().cloned())?;
        assert_eq!(valid_header.ihl(), parsed.ihl);
        assert_eq!(4, parsed.version);
        Ok(())
    }
}
