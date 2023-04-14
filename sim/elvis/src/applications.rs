//! User-level applications used to test protocols and networks.

use elvis_core::Id;

mod capture;
pub use capture::Capture;

mod send_message;
pub use send_message::SendMessage;

mod socket_client;
pub use socket_client::SocketClient;

mod socket_server;
pub use socket_server::SocketServer;

mod socket_ping_client;
pub use socket_ping_client::SocketPingClient;

mod socket_pong_server;
pub use socket_pong_server::SocketPongServer;

mod forward;
pub use forward::Forward;

mod ping_pong;
pub use ping_pong::PingPong;

mod query_tester;
pub use query_tester::QueryTester;

pub mod router;
pub use router::{Arp, Router};

mod throughput_tester;
pub use throughput_tester::ThroughputTester;

mod wait_for_message;
pub use wait_for_message::WaitForMessage;

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
