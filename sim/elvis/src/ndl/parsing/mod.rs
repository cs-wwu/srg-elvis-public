//! Mod file for parsing: allows use across Elvis

pub mod parsing_data;
mod machine_parser;
mod network_parser;
mod parser_util;
pub use parser_util::{general_parser, general_error, num_tabs_to_string};
pub use network_parser::networks_parser;
pub use machine_parser::machines_parser;
pub use parsing_data::*;