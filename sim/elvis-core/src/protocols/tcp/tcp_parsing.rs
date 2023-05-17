use crate::protocols::{ipv4::Ipv4Address, utility::Checksum};
use thiserror::Error as ThisError;

/// The number of 32-bit words in a TCP header without optional header parts
const BASE_HEADER_WORDS: u8 = 5;
/// The number of bytes in a TCP header without optional header parts
const BASE_HEADER_OCTETS: u8 = BASE_HEADER_WORDS * 4;

/// The data for a TCP header
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
        mut packet: impl Iterator<Item = u8>,
        packet_len: usize,
        src_address: Ipv4Address,
        dst_address: Ipv4Address,
    ) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { packet.next().ok_or(ParseError::HeaderTooShort) };
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

        checksum.accumulate_remainder(&mut packet);

        // Pseudo header stuff
        checksum.add_u32(src_address.into());
        checksum.add_u32(dst_address.into());
        // zero, TCP protocol number
        checksum.add_u8(0, 6);
        checksum.add_u16(
            packet_len
                .try_into()
                .map_err(|_| ParseError::PacketTooLong)?,
        );

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
    #[allow(unused)]
    pub fn bytes(&self) -> u8 {
        // Safe to do because data offset is only 4 bits
        self.data_offset * 4
    }

    /// Convert the header to its native serialized format, ready to attach to a
    /// packet and send over the wire.
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

/// An error that occurred while parsing a TCP header
#[derive(Debug, ThisError, PartialEq, Eq, Clone, Copy)]
pub enum ParseError {
    #[error("Too few bytes to constitute a TCP header")]
    HeaderTooShort,
    #[error("The packet length could not fit into a u16")]
    PacketTooLong,
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

    /// Set the window size
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
    #[allow(unused)]
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
    #[allow(unused)]
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
        mut text: impl Iterator<Item = u8>,
        text_len: usize,
    ) -> Result<TcpHeader, BuildHeaderError> {
        let mut checksum = Checksum::new();
        let length: u16 = (text_len + BASE_HEADER_OCTETS as usize)
            .try_into()
            .map_err(|_| BuildHeaderError::OverlyLongPayload)?;
        checksum.accumulate_remainder(&mut text);

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

/// An error that occurred while building a TCP header
#[derive(Debug, ThisError, PartialEq, Eq, Clone, Copy)]
pub enum BuildHeaderError {
    #[error("The TCP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}

/// The control bits of a TCP header
#[derive(Default, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Control(u8);

impl Control {
    /// Create a new Control with the given bits
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

    /// Get whether the urgent pointer field is significant
    pub const fn urg(self) -> bool {
        self.bit(5)
    }

    /// Set whether the urgent pointer field is significant
    pub fn set_urg(&mut self, state: bool) {
        self.set_bit(5, state);
    }

    /// Get whether the acknowledgment field significant
    pub const fn ack(self) -> bool {
        self.bit(4)
    }

    /// Set whether the acknowledgment field significant
    pub fn set_ack(&mut self, state: bool) {
        self.set_bit(4, state);
    }

    /// Get whether the push function is enabled
    pub const fn psh(self) -> bool {
        self.bit(3)
    }

    /// Set whether the push function is enabled
    pub fn set_psh(&mut self, state: bool) {
        self.set_bit(3, state);
    }

    /// Get whether to reset the connection
    pub const fn rst(self) -> bool {
        self.bit(2)
    }

    /// Set whether to reset the connection
    pub fn set_rst(&mut self, state: bool) {
        self.set_bit(2, state);
    }

    /// Get whether to synchronize sequence numbers
    pub const fn syn(self) -> bool {
        self.bit(1)
    }

    /// Set whether to synchronize sequence numbers
    pub fn set_syn(&mut self, state: bool) {
        self.set_bit(1, state);
    }

    /// Get whether there is no more data to send
    pub const fn fin(self) -> bool {
        self.bit(0)
    }

    /// Set whether there is no more data to send
    pub fn set_fin(&mut self, state: bool) {
        self.set_bit(0, state);
    }

    /// Get the given bit
    const fn bit(self, bit: u8) -> bool {
        (self.0 >> bit) & 0b1 == 1
    }

    /// Set the given bit
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

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Control(")?;
        let mut wrote = false;
        if self.urg() {
            wrote = true;
            write!(f, "URG")?;
        }
        if self.ack() {
            if wrote {
                write!(f, ", ")?;
            }
            wrote = true;
            write!(f, "ACK")?;
        }
        if self.psh() {
            if wrote {
                write!(f, ", ")?;
            }
            wrote = true;
            write!(f, "PSH")?;
        }
        if self.rst() {
            if wrote {
                write!(f, ", ")?;
            }
            wrote = true;
            write!(f, "RST")?;
        }
        if self.syn() {
            if wrote {
                write!(f, ", ")?;
            }
            wrote = true;
            write!(f, "SYN")?;
        }
        if self.fin() {
            if wrote {
                write!(f, ", ")?;
            }
            write!(f, "FIN")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::utility::{Endpoint, Endpoints};

    const PAYLOAD: &[u8] = b"Hello, world!";
    const SRC_ADDRESS: Ipv4Address = Ipv4Address::LOCALHOST;
    const DST_ADDRESS: Ipv4Address = Ipv4Address::SUBNET;
    const SRC_PORT: u16 = 0xcafe;
    const DST_PORT: u16 = 0xbabe;
    const SEQUENCE: u32 = 123456789;
    const WINDOW: u16 = 1024;
    const ACKNOWLEDGEMENT: u32 = 10;

    fn build_expected() -> (etherparse::TcpHeader, Vec<u8>) {
        let expected = {
            let mut expected = etherparse::TcpHeader::new(SRC_PORT, DST_PORT, SEQUENCE, WINDOW);
            expected.acknowledgment_number = ACKNOWLEDGEMENT;
            expected.ack = true;
            expected.psh = true;
            #[cfg(feature = "compute_checksum")]
            {
                let ip_header = etherparse::Ipv4Header::new(
                    PAYLOAD.len().try_into().unwrap(),
                    30,
                    etherparse::IpNumber::Tcp,
                    SRC_ADDRESS.into(),
                    DST_ADDRESS.into(),
                );
                expected.checksum = expected.calc_checksum_ipv4(&ip_header, PAYLOAD).unwrap();
            }
            expected
        };

        let serial = {
            let mut serial = vec![];
            expected.write(&mut serial).unwrap();
            serial
        };

        (expected, serial)
    }

    #[test]
    fn parses_packet() {
        let control = Control::new(false, true, true, false, false, false);

        let (expected, serial) = build_expected();

        let len = serial.len() + PAYLOAD.len();
        let actual = TcpHeader::from_bytes(
            serial.into_iter().chain(PAYLOAD.iter().cloned()),
            len,
            SRC_ADDRESS,
            DST_ADDRESS,
        )
        .unwrap();

        assert_eq!(actual.src_port, SRC_PORT);
        assert_eq!(actual.src_port, SRC_PORT);
        assert_eq!(actual.dst_port, DST_PORT);
        assert_eq!(actual.seq, SEQUENCE);
        assert_eq!(actual.ack, ACKNOWLEDGEMENT);
        assert_eq!(actual.ctl, control);
        assert_eq!(actual.wnd, WINDOW);
        assert_eq!(actual.checksum, expected.checksum);
        assert_eq!(actual.urg, 0);
        assert!(!actual.ctl.urg());
        assert!(actual.ctl.ack());
        assert!(actual.ctl.psh());
        assert!(!actual.ctl.rst());
        assert!(!actual.ctl.syn());
        assert!(!actual.ctl.fin());
    }

    #[test]
    fn builds_packet() {
        let id = Endpoints {
            local: Endpoint {
                address: Ipv4Address::LOCALHOST,
                port: 0xcafe,
            },
            remote: Endpoint {
                address: Ipv4Address::SUBNET,
                port: 0xbabe,
            },
        };

        let (_, expected) = build_expected();

        let actual = TcpHeaderBuilder::new(id.local.port, id.remote.port, SEQUENCE)
            .wnd(WINDOW)
            .psh()
            .ack(ACKNOWLEDGEMENT)
            .build(
                id.local.address,
                id.remote.address,
                PAYLOAD.iter().cloned(),
                PAYLOAD.len(),
            )
            .unwrap()
            .serialize();

        assert_eq!(expected, actual);
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
