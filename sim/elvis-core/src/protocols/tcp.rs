use thiserror::Error as ThisError;

mod tcp_parsing;

#[derive(Debug, ThisError)]
pub enum TcpError {
    #[error("Too few bytes to constitute a TCP header")]
    HeaderTooShort,
    #[error(
        "The computed checksum {actual:#06x} did not match the header checksum {expected:#06x}"
    )]
    InvalidChecksum { actual: u16, expected: u16 },
    #[error("Data offset was different from that expected for a simple header")]
    UnexpectedOptions,
    #[error("The TCP payload is longer than can fit into a single packet")]
    OverlyLongPayload,
}
