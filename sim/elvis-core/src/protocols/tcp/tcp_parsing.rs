use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};

use super::TcpError;

pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub sequence: u32,
    pub acknowledgement: u32,
    pub control: Control,
    pub window: u16,
    pub checksum: u16,
    pub urgent: u16,
}

impl TcpHeader {
    pub fn from_bytes(
        mut bytes: impl Iterator<Item = u8>,
        src_address: Ipv4Address,
        dst_address: Ipv4Address,
    ) -> Result<Self, TcpError> {
        let mut next = || -> Result<u8, TcpError> { bytes.next().ok_or(TcpError::HeaderTooShort) };
        let mut checksum = Checksum::new();

        let src_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(src_port);

        let dst_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(dst_port);

        let sequence_bytes = [next()?, next()?, next()?, next()?];
        let sequence = u32::from_be_bytes(sequence_bytes);
        checksum.add_u32(sequence_bytes);

        let acknowledgement_bytes = [next()?, next()?, next()?, next()?];
        let acknowledgement = u32::from_be_bytes(acknowledgement_bytes);
        checksum.add_u32(acknowledgement_bytes);

        let offset_reserved_control = [next()?, next()?];
        checksum.add_u16(u16::from_be_bytes(offset_reserved_control));
        let data_offset = offset_reserved_control[0] >> 4;
        let control = Control::from(offset_reserved_control[1] & 0b11_1111);

        if data_offset != 20 {
            Err(TcpError::UnexpectedOptions)?
        }

        let window = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(window);

        let expected_checksum = u16::from_be_bytes([next()?, next()?]);

        let urgent = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(urgent);

        let text_length = checksum.accumulate_remainder(&mut bytes);

        // Pseudo header stuff
        checksum.add_u32(src_address.into());
        checksum.add_u32(dst_address.into());
        // zero, TCP protocol number
        checksum.add_u8(0, 6);
        checksum.add_u16(text_length);

        let checksum = checksum.as_u16();
        if expected_checksum == checksum {
            Ok(TcpHeader {
                src_port,
                dst_port,
                sequence,
                acknowledgement,
                control,
                window,
                checksum,
                urgent,
            })
        } else {
            Err(TcpError::InvalidChecksum {
                actual: checksum,
                expected: expected_checksum,
            })
        }
    }
}

#[derive(Debug, Default, Hash, PartialEq, Eq)]
pub struct Control(u8);

impl Control {
    pub fn new(urg: bool, ack: bool, psh: bool, rst: bool, syn: bool, fin: bool) -> Self {
        Self(
            urg as u8
                | (ack as u8) << 1
                | (psh as u8) << 2
                | (rst as u8) << 3
                | (syn as u8) << 4
                | (fin as u8) << 5,
        )
    }

    /// Urgent Pointer field significant
    pub fn urg(&self) -> bool {
        self.0 & 0b1 == 1
    }

    /// Acknowledgment field significant
    pub fn ack(&self) -> bool {
        (self.0 >> 1) & 0b1 == 1
    }

    /// Push Function
    pub fn psh(&self) -> bool {
        (self.0 >> 2) & 0b1 == 1
    }

    /// Reset the connection
    pub fn rst(&self) -> bool {
        (self.0 >> 3) & 0b1 == 1
    }

    /// Synchronize sequence numbers
    pub fn syn(&self) -> bool {
        (self.0 >> 4) & 0b1 == 1
    }

    /// No more data from sender
    pub fn fin(&self) -> bool {
        (self.0 >> 5) & 0b1 == 1
    }
}

impl From<u8> for Control {
    fn from(n: u8) -> Self {
        Self(n)
    }
}

impl From<Control> for u8 {
    fn from(control: Control) -> Self {
        control.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_packet() -> anyhow::Result<()> {
        let payload = b"Hello, world!";
        let ttl = 30;
        let src_address = Ipv4Address::LOCALHOST;
        let dst_address = Ipv4Address::SUBNET;
        let src_port = 0xcafe;
        let dst_port = 0xbabe;
        let sequence = 123456789;
        let window = 1024;
        let acknowledgement = 10;
        let control = Control::new(false, true, true, false, false, false);
        let mut expected = etherparse::TcpHeader::new(src_port, dst_port, sequence, window);
        expected.acknowledgment_number = acknowledgement;
        expected.ack = true;
        expected.psh = true;
        let ip_header = etherparse::Ipv4Header::new(
            payload.len().try_into()?,
            ttl,
            etherparse::IpNumber::Tcp,
            src_address.into(),
            dst_address.into(),
        );
        expected.checksum = expected.calc_checksum_ipv4(&ip_header, payload)?;
        let serial = {
            let mut serial = vec![];
            expected.write(&mut serial)?;
            serial
        };
        let actual = TcpHeader::from_bytes(serial.iter().cloned(), src_address, dst_address)?;
        assert_eq!(actual.src_port, src_port);
        assert_eq!(actual.src_port, src_port);
        assert_eq!(actual.dst_port, dst_port);
        assert_eq!(actual.sequence, sequence);
        assert_eq!(actual.acknowledgement, acknowledgement);
        assert_eq!(actual.control, control);
        assert_eq!(actual.window, window);
        assert_eq!(actual.checksum, expected.checksum);
        assert_eq!(actual.urgent, 0);
        Ok(())
    }
}
