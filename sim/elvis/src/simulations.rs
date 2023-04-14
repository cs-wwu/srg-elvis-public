//! Various prebuilt simulation setups for testing, benchmarking, and examples.

mod basic;
pub use basic::basic;

mod telephone_multi;
pub use telephone_multi::telephone_multi;

mod telephone_single;
pub use telephone_single::telephone_single;

mod ping_pong;
pub use ping_pong::ping_pong;

mod query;
pub use query::query;

mod udp_gigabyte_bench;
pub use udp_gigabyte_bench::udp_gigabyte_bench;

mod router_single;
pub use router_single::router_single;

mod router_multi;
pub use router_multi::router_multi;

// mod socket_basic;
// pub use socket_basic::socket_basic;

// mod latency;
// pub use latency::latency;

// mod throughput;
// pub use throughput::throughput;

mod tcp_with_reliable;
pub use tcp_with_reliable::tcp_with_reliable;

// mod tcp_with_unreliable;
// pub use tcp_with_unreliable::tcp_with_unreliable;

// pub mod subscribing;

// mod tcp_gigabyte_bench;
// pub use tcp_gigabyte_bench::tcp_gigabyte_bench;
