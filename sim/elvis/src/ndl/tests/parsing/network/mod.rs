//! Tests for network parsing
#![cfg(test)]

use crate::ndl::core_parser;

#[test]
fn non_ip() {
    let result = core_parser(include_str!("non_ip.ndl"));
    let s= "Line 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \n\t\tLine 3: expected type IP and got type Network instead.\n\n";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn invalid_ip_indent() {
    let result = core_parser(include_str!("invalid_ip_indent.ndl"));
    let s= "Line 1: Unable to parse inside of Networks due to: \n\tLine 3: Unable to parse inside of Network due to: \n\t\tLine 3: expected 2 tabs and got 3 tabs instead.\n\n";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn empty_networks() {
    let result = core_parser(include_str!("empty_networks.ndl"));
    let s= "Line 1: Unable to parse inside of Networks due to: \n\tLine 4: expected type Network and got type Networks instead.\n\n";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn invalid_type() {
    let result = core_parser(include_str!("invalid_type.ndl"));
    let s: String = "Line 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \n\t\tLine 5: extra argument at 'S ip='192.168.1.121''\n\n".to_string();
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn duplicate_id() {
    let result = core_parser(include_str!("duplicate_id.ndl"));
    let s = "Line 1: Unable to insert Network into Networks due to duplicate id: 5\n";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn outoforder_duplicate_id() {
    let result = core_parser(include_str!("outoforder_duplicate_id.ndl"));
    let s = "Line 21: Unable to insert Network into Networks due to duplicate id: 5";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}
