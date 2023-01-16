//! Mod file for parsing: allows use across Elvis

mod machine_parser;
mod network_parser;
pub mod parser;
mod parser_util;
pub mod parsing_data;
pub use machine_parser::machines_parser;
pub use network_parser::networks_parser;
pub use parser::core_parser;
pub use parser_util::{general_error, general_parser, num_tabs_to_string};
pub use parsing_data::*;
