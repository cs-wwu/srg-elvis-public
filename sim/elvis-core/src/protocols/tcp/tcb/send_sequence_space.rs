//      1         2          3          4
// ----------|----------|----------|----------
//        SND.UNA    SND.NXT    SND.UNA
//                             +SND.WND
//
// 1 - old sequence numbers which have been acknowledged
// 2 - sequence numbers of unacknowledged data
// 3 - sequence numbers allowed for new data transmission (send window)
// 4 - future sequence numbers which are not yet allowed
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct SendSequenceSpace {
    /// Oldest unacknowledged sequence number
    pub una: u32,
    /// Next sequence number to be sent
    pub nxt: u32,
    /// The size of the remote TCP's window
    pub wnd: u16,
    /// Segment sequence number used for last window update
    pub wl1: u32,
    /// Segment acknowledgment number used for last window update
    pub wl2: u32,
    /// Initial send sequence number
    pub iss: u32,
}
