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

mod latency;
pub use latency::latency;

mod throughput;
pub use throughput::throughput;

mod router_simulation;
pub use router_simulation::router_simulation;