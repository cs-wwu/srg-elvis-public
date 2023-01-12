use elvis::parsing::core_parser;
use elvis::parsing::parsing_data::*;

/// main wrapper for parsing testing.
pub fn parser_testing(file_path: &str) -> Result<Sim, String> {
    let res = core_parser(file_path.to_string());
    match res {
        Ok(s) => {
            return Ok(s);
        }

        Err(e) => {
            return Err(e);
        }
    }
}
#[test]
fn parsing_network_fail_non_ip() {
    let result = parser_testing("./tests/parsing_tests/network_fail_non_ip.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_non_ip.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \n\t\tLine 3: expected type IP and got type Network instead.\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn parsing_network_fail_invalid_ip_indent() {
    let result = parser_testing("./tests/parsing_tests/network_fail_invalid_ip_indent.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_invalid_ip_indent.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 3: Unable to parse inside of Network due to: \n\t\tLine 3: expected 2 tabs and got 3 tabs instead.\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn parsing_network_fail_empty_networks() {
    let result = parser_testing("./tests/parsing_tests/network_fail_empty_networks.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_empty_networks.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 4: expected type Network and got type Networks instead.\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

//4
#[test]
fn parsing_network_fail_invalid_type() {
    let result = parser_testing("./tests/parsing_tests/network_fail_invalid_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_invalid_type.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \nLine 5: extra argument at 'S ip='192.168.1.121''\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn parsing_network_fail_duplicate_id() {
    let result = parser_testing("./tests/parsing_tests/network_fail_duplicate_id.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_duplicate_id.txt:\n\nLine 1: Unable to insert Network into Networks due to duplicate id: 5\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}

#[test]
fn parsing_network_fail_outoforder_duplicate_id() {
    let result = parser_testing("./tests/parsing_tests/network_fail_outoforder_duplicate_id.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_fail_outoforder_duplicate_id.txt:\n\nLine 21: Unable to insert Network into Networks due to duplicate id: 5".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}
