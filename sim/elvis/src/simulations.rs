//! Various prebuilt simulation setups for testing, benchmarking, and examples.

mod basic;
pub use basic::basic;

mod latent;
pub use latent::latent;

mod telephone_multi;
pub use telephone_multi::telephone_multi;

mod telephone_single;
pub use telephone_single::telephone_single;

mod unreliable;
pub use unreliable::unreliable;

mod ping_pong;
pub use ping_pong::ping_pong;

mod query;
pub use query::query;
