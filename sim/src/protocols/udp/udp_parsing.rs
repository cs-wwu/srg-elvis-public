use super::udp_misc::UdpError;

pub(super) struct UdpHeader {}

impl UdpHeader {
    pub fn from_bytes(bytes: impl Iterator<Item = u8>) -> Result<Self, UdpError> {
        Ok(Self {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_udp_header() {
        let payload = b"Hello, world!";
        let time_to_live = 30;
        let protocol = etherparse::IpNumber::Udp;
        let source = [127, 0, 0, 1];
        let destination = [123, 45, 67, 89];
        let ip_header = etherparse::Ipv4Header::new(
            payload.len().try_into().unwrap(),
            time_to_live,
            protocol,
            source,
            destination,
        );
        let source_port = 12345u16;
        let destination_port = 6789u16;
        let valid_header = etherparse::UdpHeader::with_ipv4_checksum(
            source_port,
            destination_port,
            &ip_header,
            payload,
        )
        .unwrap();
        let serial = {
            let mut serial = vec![];
            valid_header.write(&mut serial);
            serial
        };
        let actual = UdpHeader::from_bytes(serial.into_iter()).unwrap();
    }
}
