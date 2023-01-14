//! Various methods related to parsing NDL
mod generating;
mod ndl;
pub mod parsing;
pub use ndl::core_parser;
pub use ndl::generate_sim;
