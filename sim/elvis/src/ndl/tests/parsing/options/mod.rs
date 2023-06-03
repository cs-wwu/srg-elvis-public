//! Tests for options parsing
#![cfg(test)]

use crate::ndl::core_parser;

#[test]
fn duplicate_argument() {
    let result = core_parser(include_str!("duplicate_argument.ndl"));
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            let s = "Line 1: Unable to parse inside of Networks due to: \n\tLine 6: duplicate argument 'id'='5'\n\n";
            assert_eq!(&e, s);
        }
    }
}
