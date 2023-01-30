//! User-level applications used to test protocols and networks.

use elvis_core::Id;

mod capture;
pub use capture::Capture;

mod send_message;
pub use send_message::SendMessage;

mod forward;
pub use forward::Forward;

mod ping_pong;
pub use ping_pong::PingPong;

mod query_tester;
pub use query_tester::QueryTester;

mod throughput_tester;
pub use throughput_tester::ThroughputTester;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    #[default]
    Udp = 17,
    Tcp = 6,
}

impl Transport {
    pub fn id(&self) -> Id {
        Id::new(*self as u64)
    }
}
