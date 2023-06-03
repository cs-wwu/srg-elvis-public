//! Various methods related to parsing NDL
mod generating;
pub mod parsing;
mod sim_creator;
pub use crate::ndl::parsing::core_parser;
pub use sim_creator::generate_and_run_sim;
mod tests;
