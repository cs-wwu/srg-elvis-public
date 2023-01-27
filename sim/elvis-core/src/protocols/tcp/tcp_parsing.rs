use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};
use thiserror::Error as ThisError;

const BASE_HEADER_WORDS: u8 = 5;
const BASE_HEADER_OCTETS: u8 = BASE_HEADER_WORDS * 4;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct TcpHeader {
    /// The source port number
    pub src_port: u16,
    /// The destination port number
    pub dst_port: u16,
    /// The sequence number of the first data octet in this segment (except when
    /// SYN is present). If SYN is present the sequence number is the initial
    /// sequence number (ISN) and the first data octet is ISN+1.
    pub seq: u32,
    /// If the ACK control bit is set this field contains the value of the next
    /// sequence number the sender of the segment is expecting to receive. Once
    /// a connection is established this is always sent.
    pub ack: u32,
    /// The number of 32-bit words in the TCP header
    pub data_offset: u8,
    /// Flags that adjust the how segments are handled
    pub ctl: Control,
    /// The number of data octets beginning with the one indicated in the
    /// acknowledgment field which the sender of this segment is willing to
    /// accept.
    pub wnd: u16,
    /// This field communicates the current value of the urgent pointer as a
    /// positive offset from the sequence number in this segment. The urgent
    /// pointer points to the sequence number of the octet following the urgent
    /// data. This field is only be interpreted in segments with the URG
    /// control bit set.
    pub urg: u16,
    /// The header checksum
    pub checksum: u16,
}

impl TcpHeader {
    /// Parses a serialized TCP header into its constituent fields.
    pub fn from_bytes(
        mut bytes: impl Iterator<Item = u8>,
        src_address: Ipv4Address,
        dst_address: Ipv4Address,
    ) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };
        let mut checksum = Checksum::new();

        let src_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(src_port);

        let dst_port = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(dst_port);

        let sequence_bytes = [next()?, next()?, next()?, next()?];
        let seq = u32::from_be_bytes(sequence_bytes);
        checksum.add_u32(sequence_bytes);

        let acknowledgement_bytes = [next()?, next()?, next()?, next()?];
        let ack = u32::from_be_bytes(acknowledgement_bytes);
        checksum.add_u32(acknowledgement_bytes);

        let offset_reserved_control = [next()?, next()?];
        checksum.add_u16(u16::from_be_bytes(offset_reserved_control));
        let data_offset = offset_reserved_control[0] >> 4;
        let ctl = Control::from(offset_reserved_control[1] & 0b11_1111);

        if data_offset != BASE_HEADER_WORDS {
            // TODO(hardint): Support optional headers
            Err(ParseError::UnexpectedOptions)?
        }

        let wnd = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(wnd);

        let expected_checksum = u16::from_be_bytes([next()?, next()?]);

        let urg = u16::from_be_bytes([next()?, next()?]);
        checksum.add_u16(urg);

        let text_length = checksum.accumulate_remainder(&mut bytes);
        let tcp_length = text_length + data_offset as u16 * 4;

        // Pseudo header stuff
        checksum.add_u32(src_address.into());
        checksum.add_u32(dst_address.into());
        // zero, TCP protocol number
        checksum.add_u8(0, 6);
        checksum.add_u16(tcp_length);

        let checksum = checksum.as_u16();
        if expected_checksum == checksum {
            Ok(TcpHeader {
                src_port,
                dst_port,
                seq,
                ack,
                data_offset,
                ctl,
                wnd,
                urg,
                checksum,
            })
        } else {
            Err(ParseError::Checksum {
                actual: checksum,
                expected: expected_checksum,
            })
        }
    }

    /// Size of the header in bytes
    pub fn bytes(&self) -> u8 {
        // Safe to do because data offset is only 4 bits
        self.data_offset * 4
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(BASE_HEADER_OCTETS as usize);
        out.extend_from_slice(&self.src_port.to_be_bytes());
        out.extend_from_slice(&self.dst_port.to_be_bytes());
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&self.ack.to_be_bytes());
        out.push(self.data_offset << 4);
        out.push(self.ctl.into());
        out.extend_from_slice(&self.wnd.to_be_bytes());
        out.extend_from_slice(&self.checksum.to_be_bytes());
        out.extend_from_slice(&self.urg.to_be_bytes());
        out
    }
}

#[derive(Debug, ThisError, PartialEq, Eq, Clone, Copy)]
pub enum ParseError {
    #[error("Too few bytes to constitute a TCP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    Checksum { actual: u16, expected: u16 },
    #[error("Data offset was different from that expected for a simple header")]
    UnexpectedOptions,
}

/// Used for building a serialized TCP header
#[derive(Debug)]
pub struct TcpHeaderBuilder(TcpHeader);

impl TcpHeaderBuilder {
    /// Initialize the TCP header with defaults and the given values
    pub fn new(src_port: u16, dst_port: u16, seq: u32) -> Self {
        Self(TcpHeader {
            src_port,
            dst_port,
            seq,
            wnd: 0,
            ack: 0,
            urg: 0,
            ctl: Control::default(),

            // Filled in by .build()
            data_offset: 0,
            checksum: 0,
        })
    }

    pub fn wnd(mut self, wnd: u16) -> Self {
        self.0.wnd = wnd;
        self
    }

    /// Set the acknowledgement number
    pub fn ack(mut self, ack: u32) -> Self {
        self.0.ack = ack;
        self.0.ctl.set_ack(true);
        self
    }

    /// Set the psh bit up
    pub fn psh(mut self) -> Self {
        self.0.ctl.set_psh(true);
        self
    }

    /// Set the rst bit up
    pub fn rst(mut self) -> Self {
        self.0.ctl.set_rst(true);
        self
    }

    /// Set the syn bit up
    pub fn syn(mut self) -> Self {
        self.0.ctl.set_syn(true);
        self
    }

    /// Set the fin bit up
    pub fn fin(mut self) -> Self {
        self.0.ctl.set_fin(true);
        self
    }

    /// Set urgent pointer
    pub fn urg(mut self, urg: u16) -> Self {
        self.0.ctl.set_urg(true);
        self.0.urg = urg;
        self
    }

    /// Get the serialized header
    pub fn build(
        self,
        src_address: Ipv4Address,
        dst_address: Ipv4Address,
        mut payload: impl Iterator<Item = u8>,
    ) -> Result<TcpHeader, BuildHeaderError> {
        let mut checksum = Checksum::new();
        let length = checksum
            .accumulate_remainder(&mut payload)
            .checked_add(BASE_HEADER_OCTETS as u16)
            .ok_or(BuildHeaderError::OverlyLongPayload)?;

        // TODO(hardint): Should change when header options are supported
        let data_offset = BASE_HEADER_WORDS;

        // Pseudo header
        checksum.add_u32(src_address.into());
        checksum.add_u32(dst_address.into());
        checksum.add_u8(0, 6);
        checksum.add_u16(length);

        // Header parts
        checksum.add_u16(self.0.src_port);
        checksum.add_u16(self.0.dst_port);
        checksum.add_u32(self.0.seq.to_be_bytes());
        checksum.add_u32(self.0.ack.to_be_bytes());
        checksum.add_u8(data_offset << 4, self.0.ctl.into());
        checksum.add_u16(self.0.wnd);
        checksum.add_u16(self.0.urg);

        let mut header = self.0;
        header.data_offset = data_offset;
        header.checksum = checksum.as_u16();
        Ok(header)
    }
}

#[derive(Debug, ThisError, PartialEq, Eq, Clone, Copy)]
pub enum BuildHeaderError {
    #[error("The TCP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}

/// The control bits of a TCP header
#[derive(Debug, Default, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Control(u8);

impl Control {
    pub const fn new(urg: bool, ack: bool, psh: bool, rst: bool, syn: bool, fin: bool) -> Self {
        Self(
            fin as u8
                | (syn as u8) << 1
                | (rst as u8) << 2
                | (psh as u8) << 3
                | (ack as u8) << 4
                | (urg as u8) << 5,
        )
    }

    /// Urgent Pointer field significant
    pub const fn urg(self) -> bool {
        self.bit(5)
    }

    pub fn set_urg(&mut self, state: bool) {
        self.set_bit(5, state);
    }

    /// Acknowledgment field significant
    pub const fn ack(self) -> bool {
        self.bit(4)
    }

    pub fn set_ack(&mut self, state: bool) {
        self.set_bit(4, state);
    }

    /// Push Function
    pub const fn psh(self) -> bool {
        self.bit(3)
    }

    pub fn set_psh(&mut self, state: bool) {
        self.set_bit(3, state);
    }

    /// Reset the connection
    pub const fn rst(self) -> bool {
        self.bit(2)
    }

    pub fn set_rst(&mut self, state: bool) {
        self.set_bit(2, state);
    }

    /// Synchronize sequence numbers
    pub const fn syn(self) -> bool {
        self.bit(1)
    }

    pub fn set_syn(&mut self, state: bool) {
        self.set_bit(1, state);
    }

    /// No more data from sender
    pub const fn fin(self) -> bool {
        self.bit(0)
    }

    pub fn set_fin(&mut self, state: bool) {
        self.set_bit(0, state);
    }

    const fn bit(self, bit: u8) -> bool {
        (self.0 >> bit) & 0b1 == 1
    }

    fn set_bit(&mut self, bit: u8, state: bool) {
        self.0 = (self.0 & !(1 << bit)) | ((state as u8) << bit);
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
    use crate::protocols::{tcp::ConnectionId, utility::Socket};

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

        let expected = {
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
            expected
        };

        let serial = {
            let mut serial = vec![];
            expected.write(&mut serial)?;
            serial
        };

        let actual = TcpHeader::from_bytes(
            serial.into_iter().chain(payload.iter().cloned()),
            src_address,
            dst_address,
        )?;

        assert_eq!(actual.src_port, src_port);
        assert_eq!(actual.src_port, src_port);
        assert_eq!(actual.dst_port, dst_port);
        assert_eq!(actual.seq, sequence);
        assert_eq!(actual.ack, acknowledgement);
        assert_eq!(actual.ctl, control);
        assert_eq!(actual.wnd, window);
        assert_eq!(actual.checksum, expected.checksum);
        assert_eq!(actual.urg, 0);
        assert!(!actual.ctl.urg());
        assert!(actual.ctl.ack());
        assert!(actual.ctl.psh());
        assert!(!actual.ctl.rst());
        assert!(!actual.ctl.syn());
        assert!(!actual.ctl.fin());
        Ok(())
    }

    #[test]
    fn builds_packet() -> anyhow::Result<()> {
        let payload = b"Hello, world!";
        let ttl = 30;
        let sequence = 123456789;
        let window = 1024;
        let acknowledgement = 10;

        let id = ConnectionId {
            local: Socket {
                address: Ipv4Address::LOCALHOST,
                port: 0xcafe,
            },
            remote: Socket {
                address: Ipv4Address::SUBNET,
                port: 0xbabe,
            },
        };

        let expected = {
            let mut expected =
                etherparse::TcpHeader::new(id.local.port, id.remote.port, sequence, window);
            expected.acknowledgment_number = acknowledgement;
            expected.ack = true;
            expected.psh = true;
            let ip_header = etherparse::Ipv4Header::new(
                payload.len().try_into()?,
                ttl,
                etherparse::IpNumber::Tcp,
                id.local.address.into(),
                id.remote.address.into(),
            );
            expected.checksum = expected.calc_checksum_ipv4(&ip_header, payload)?;
            expected
        };

        let expected = {
            let mut serial = vec![];
            expected.write(&mut serial)?;
            serial
        };

        let actual = TcpHeaderBuilder::new(id.local.port, id.remote.port, sequence)
            .wnd(window)
            .psh()
            .ack(acknowledgement)
            .build(id.local.address, id.remote.address, payload.iter().cloned())?
            .serialize();

        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn control_works() {
        let control = Control::new(true, false, true, false, true, false);
        assert!(control.urg());
        assert!(!control.ack());
        assert!(control.psh());
        assert!(!control.rst());
        assert!(control.syn());
        assert!(!control.fin());

        let control = {
            let mut control = Control::default();
            control.set_ack(true);
            control.set_rst(true);
            control.set_fin(true);
            control
        };
        assert!(!control.urg());
        assert!(control.ack());
        assert!(!control.psh());
        assert!(control.rst());
        assert!(!control.syn());
        assert!(control.fin());
    }
}
