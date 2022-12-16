//! Various methods related to parsing NDL
mod parser;
pub mod parsing_data;
mod machine_parser;
mod network_parser;
mod core_parser;
pub use parser::{generate_sim, core_parser};
pub use core_parser::{general_parser, general_error, num_tabs_to_string};
pub use network_parser::networks_parser;
pub use machine_parser::machines_parser;
pub use parsing_data::*;
