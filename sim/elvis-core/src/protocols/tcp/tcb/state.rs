/// The state of the TCP state machine as described in section 3.3.2. The CLOSED
/// and LISTEN states are not included and are instead handled by the
/// freestanding functions [`segment_arrives_closed`](super::segment_arrives_closed) and
/// [`segment_arrives_listen`](super::segment_arrives_listen). The TCP state machine is described
/// by the diagram below.
///
/// ```text
///                             +---------+ ---------\      active OPEN
///                             |  CLOSED |            \    -----------
///                             +---------+<---------\   \   create TCB
///                               |     ^              \   \  snd SYN
///                  passive OPEN |     |   CLOSE        \   \
///                  ------------ |     | ----------       \   \
///                   create TCB  |     | delete TCB         \   \
///                               V     |                      \   \
///           rcv RST (note 1)  +---------+            CLOSE    |    \
///        -------------------->|  LISTEN |          ---------- |     |
///       /                     +---------+          delete TCB |     |
///      /           rcv SYN      |     |     SEND              |     |
///     /           -----------   |     |    -------            |     V
/// +--------+      snd SYN,ACK  /       \   snd SYN          +--------+
/// |        |<-----------------           ------------------>|        |
/// |  SYN   |                    rcv SYN                     |  SYN   |
/// |  RCVD  |<-----------------------------------------------|  SENT  |
/// |        |                  snd SYN,ACK                   |        |
/// |        |------------------           -------------------|        |
/// +--------+   rcv ACK of SYN  \       /  rcv SYN,ACK       +--------+
///    |         --------------   |     |   -----------
///    |                x         |     |     snd ACK
///    |                          V     V
///    |  CLOSE                 +---------+
///    | -------                |  ESTAB  |
///    | snd FIN                +---------+
///    |                 CLOSE    |     |    rcv FIN
///    V                -------   |     |    -------
/// +---------+         snd FIN  /       \   snd ACK         +---------+
/// |  FIN    |<----------------          ------------------>|  CLOSE  |
/// | WAIT-1  |------------------                            |   WAIT  |
/// +---------+          rcv FIN  \                          +---------+
///   | rcv ACK of FIN   -------   |                          CLOSE  |
///   | --------------   snd ACK   |                         ------- |
///   V        x                   V                         snd FIN V
/// +---------+               +---------+                    +---------+
/// |FINWAIT-2|               | CLOSING |                    | LAST-ACK|
/// +---------+               +---------+                    +---------+
///   |              rcv ACK of FIN |                 rcv ACK of FIN |
///   |  rcv FIN     -------------- |    Timeout=2MSL -------------- |
///   |  -------            x       V    ------------        x       V
///    \ snd ACK              +---------+delete TCB          +---------+
///      -------------------->|TIME-WAIT|------------------->| CLOSED  |
///                           +---------+                    +---------+
/// ```
/// Figure 5: TCP Connection State Diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// Waiting for a matching connection request after having sent a connection
    /// request.
    SynSent,
    /// Waiting for a confirming connection request acknowledgment after having
    /// both received and sent a connection request.
    SynReceived,
    /// An open connection, data received can be delivered to the user. The
    /// normal state for the data transfer phase of the connection.
    Established,
    /// Waiting for a connection termination request from the remote TCP, or an
    /// acknowledgment of the connection termination request previously sent.
    FinWait1,
    /// Waiting for a connection termination request from the remote TCP.
    FinWait2,
    /// Waiting for a connection termination request from the local user.
    CloseWait,
    /// Waiting for a connection termination request acknowledgment from the
    /// remote TCP.
    Closing,
    /// Waiting for an acknowledgment of the connection termination request
    /// previously sent to the remote TCP (which includes an acknowledgment of
    /// its connection termination request).
    LastAck,
    /// Waiting for enough time to pass to be sure the remote TCP received the
    /// acknowledgment of its connection termination request.
    TimeWait,
}
