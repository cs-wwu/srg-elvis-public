//! Tests for machine parsing
#![cfg(test)]

use crate::ndl::core_parser;

#[test]
fn no_applications() {
    let result = core_parser(include_str!("no_applications.ndl"));
    let s= "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tFailed to include all required types for machine. Still needs types: [Applications]\n";
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
fn invalid_machine_indent() {
    let result = core_parser(include_str!("invalid_machine_indent.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Invalid tab count. Expected 1 tabs, got 2 tabs.\n\n";
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
fn invalid_network_declaration() {
    let result = core_parser(include_str!("invalid_network_declaration.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 11: Unable to parse inside of Networks due to: \n\t\t\tLine 12: expected type Network and got type IP instead.\n\n";
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
fn invalid_machine_declaration() {
    let result = core_parser(include_str!("invalid_machine_declaration.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 18: expected type Machine and got type Network instead.\n\n";
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
fn invalid_networks_type() {
    let result = core_parser(include_str!("invalid_networks_type.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 11: Unexpected type Network.\n\n";
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
fn invalid_protocols_type() {
    let result = core_parser(include_str!("invalid_protocols_type.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 13: Unexpected type Protocol.\n\n";
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
fn invalid_protocol_type() {
    let result = core_parser(include_str!("invalid_protocol_type.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 13: Unable to parse inside of Protocols due to: \n\t\t\tLine 14: expected type Protocol and got type Protocols instead.\n\n";
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
fn invalid_applications_type() {
    let result = core_parser(include_str!("invalid_applications_type.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 16: Unexpected type Application.\n\n";
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
fn invalid_application_type() {
    let result = core_parser(include_str!("invalid_application_type.ndl"));
    let s = "Line 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 16: Unable to parse inside of Applications due to: \n\t\t\tLine 16: expected type Application and got type Network instead.\n\n";
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
fn invalid_network_args() {
    let result = core_parser(include_str!("invalid_network_args.ndl"));
    let s = "Line 8: Unable to parse inside of Machines due to: \n\tLine 9: Unable to parse inside of Machine due to: \n\t\tLine 10: Unable to parse inside of Networks due to: \n\t\t\tLine 11: extra argument at ' id='5'''\n\n";
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
fn invalid_protocol_args() {
    let result = core_parser(include_str!("invalid_protocol_args.ndl"));
    let s = "Line 8: Unable to parse inside of Machines due to: \n\tLine 9: Unable to parse inside of Machine due to: \n\t\tLine 13: Unable to parse inside of Protocols due to: \n\t\t\tLine 14: extra argument at ' name='IPv4'''\n\n";
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
fn invalid_application_args() {
    let result = core_parser(include_str!("invalid_application_args.ndl"));
    let s = "Line 8: Unable to parse inside of Machines due to: \n\tLine 9: Unable to parse inside of Machine due to: \n\t\tLine 16: Unable to parse inside of Applications due to: \n\t\t\tLine 17: extra argument at ' name='send_message'' message='Hello this is an awesome test message!' to='recv1' port='0xbeef''\n\n";
    match result {
        Ok(_s) => {
            panic!();
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}
