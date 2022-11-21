use thiserror::Error as ThisError;

mod tcp_parsing;

/// The maintenance of a TCP connection requires the remembering of several
/// variables.  We conceive of these variables being stored in a connection
/// record called a Transmission Control Block or TCB.  Among the variables
/// stored in the TCB are the local and remote socket numbers, the security and
/// precedence of the connection, pointers to the user's send and receive
/// buffers, pointers to the retransmit queue and to the current segment. In
/// addition several variables relating to the send and receive sequence numbers
/// are stored in the TCB.
///
/// Send Sequence Variables
///     SND.UNA - send unacknowledged
///     SND.NXT - send next
///     SND.WND - send window
///     SND.UP  - send urgent pointer
///     SND.WL1 - segment sequence number used for last window update
///     SND.WL2 - segment acknowledgment number used for last window
///               update
///     ISS     - initial send sequence number
///
/// Receive Sequence Variables
///     RCV.NXT - receive next
///     RCV.WND - receive window
///     RCV.UP  - receive urgent pointer
///     IRS     - initial receive sequence number
///
/// The following diagrams may help to relate some of these variables to
/// the sequence space.
///
/// Send Sequence Space
///
///                   1         2          3          4
///              ----------|----------|----------|----------
///                     SND.UNA    SND.NXT    SND.UNA
///                                          +SND.WND
///
///        1 - old sequence numbers which have been acknowledged
///        2 - sequence numbers of unacknowledged data
///        3 - sequence numbers allowed for new data transmission
///        4 - future sequence numbers which are not yet allowed
///
/// The send window is the portion of the sequence space labeled 3 in
/// figure 4.
///
/// Receive Sequence Space
///
///                       1          2          3
///                   ----------|----------|----------
///                          RCV.NXT    RCV.NXT
///                                    +RCV.WND
///
///        1 - old sequence numbers which have been acknowledged
///        2 - sequence numbers allowed for new reception
///        3 - future sequence numbers which are not yet allowed
///
/// The receive window is the portion of the sequence space labeled 2 in
/// figure 5.
///
///
/// There are also some variables used frequently in the discussion that
/// take their values from the fields of the current segment.
///
/// Current Segment Variables
///     SEG.SEQ - segment sequence number
///     SEG.ACK - segment acknowledgment number
///     SEG.LEN - segment length
///     SEG.WND - segment window
///     SEG.UP  - segment urgent pointer
///     SEG.PRC - segment precedence value
struct Tcb;

///                              +---------+ ---------\      active OPEN
///                              |  CLOSED |            \    -----------
///                              +---------+<---------\   \   create TCB
///                                |     ^              \   \  snd SYN
///                   passive OPEN |     |   CLOSE        \   \
///                   ------------ |     | ----------       \   \
///                    create TCB  |     | delete TCB         \   \
///                                V     |                      \   \
///                              +---------+            CLOSE    |    \
///                              |  LISTEN |          ---------- |     |
///                              +---------+          delete TCB |     |
///                   rcv SYN      |     |     SEND              |     |
///                  -----------   |     |    -------            |     V
/// +---------+      snd SYN,ACK  /       \   snd SYN          +---------+
/// |         |<-----------------           ------------------>|         |
/// |   SYN   |                    rcv SYN                     |   SYN   |
/// |   RCVD  |<-----------------------------------------------|   SENT  |
/// |         |                    snd ACK                     |         |
/// |         |------------------           -------------------|         |
/// +---------+   rcv ACK of SYN  \       /  rcv SYN,ACK       +---------+
///   |           --------------   |     |   -----------
///   |                  x         |     |     snd ACK
///   |                            V     V
///   |  CLOSE                   +---------+
///   | -------                  |  ESTAB  |
///   | snd FIN                  +---------+
///   |                   CLOSE    |     |    rcv FIN
///   V                  -------   |     |    -------
/// +---------+          snd FIN  /       \   snd ACK          +---------+
/// |  FIN    |<-----------------           ------------------>|  CLOSE  |
/// | WAIT-1  |------------------                              |   WAIT  |
/// +---------+          rcv FIN  \                            +---------+
///   | rcv ACK of FIN   -------   |                            CLOSE  |
///   | --------------   snd ACK   |                           ------- |
///   V        x                   V                           snd FIN V
/// +---------+                  +---------+                   +---------+
/// |FINWAIT-2|                  | CLOSING |                   | LAST-ACK|
/// +---------+                  +---------+                   +---------+
///   |                rcv ACK of FIN |                 rcv ACK of FIN |
///   |  rcv FIN       -------------- |    Timeout=2MSL -------------- |
///   |  -------              x       V    ------------        x       V
///    \ snd ACK                 +---------+delete TCB         +---------+
///     ------------------------>|TIME WAIT|------------------>| CLOSED  |
///                              +---------+                   +---------+
enum ConnectionState {
    /// represents waiting for a connection request from any remote TCP and
    /// port.
    Listen,
    /// represents waiting for a matching connection request after having sent a
    /// connection request.
    SynSent,
    /// represents waiting for a confirming connection request acknowledgment
    /// after having both received and sent a connection request.
    SynReceived,
    /// represents an open connection, data received can be delivered to the
    /// user. The normal state for the data transfer phase of the connection.
    Established,
    /// represents waiting for a connection termination request from the remote
    /// TCP, or an acknowledgment of the connection termination request
    /// previously sent.
    FinWait1,
    /// represents waiting for a connection termination request from the remote
    /// TCP.
    FinWait2,
    /// represents waiting for a connection termination request from the local
    /// user.
    CloseWait,
    /// represents waiting for a connection termination request acknowledgment
    /// from the remote TCP.
    Closing,
    /// represents waiting for an acknowledgment of the connection termination
    /// request previously sent to the remote TCP (which includes an
    /// acknowledgment of its connection termination request).
    LastAck,
    /// represents waiting for enough time to pass to be sure the remote TCP
    /// received the acknowledgment of its connection termination request.
    TimeWait,
}

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
