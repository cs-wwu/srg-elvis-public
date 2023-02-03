use crate::{protocols::tcp::tcp_parsing::TcpHeader, Message};

#[derive(Debug, Clone)]
pub struct Segment {
    pub header: TcpHeader,
    pub text: Message,
}

impl Segment {
    pub fn new(seg: TcpHeader, message: Message) -> Self {
        Self {
            header: seg,
            text: message,
        }
    }

    /// The length of the segment data, including any control bits
    pub fn seg_len(&self) -> usize {
        self.text.len() + self.header.ctl.syn() as usize + self.header.ctl.fin() as usize
    }

    pub fn into_inner(self) -> (TcpHeader, Message) {
        (self.header, self.text)
    }
}
