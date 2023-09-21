//! Mod file for parsing: allows use across Elvis

mod machine_parser;
mod network_parser;
pub mod parser;
mod parser_util;
pub mod parsing_data;
use machine_parser::machines_parser;
use network_parser::networks_parser;
pub use parser::core_parser;
use parser_util::{general_parser, num_tabs_to_string};
pub use parsing_data::*;
