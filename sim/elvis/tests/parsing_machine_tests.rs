use elvis::parsing::core_parser;
use elvis::parsing::parsing_data::*;

fn parser_testing(file_path: &str) -> Result<Sim, String> {
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
fn parsing_machine_fail_no_applications() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_no_applications.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_no_applications.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tFailed to include all required types for machine. Still needs types: [Applications]\n".to_string();
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
fn parsing_machine_fail_invalid_machine_indent() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_machine_indent.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_machine_indent.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Invalid tab count. Expected 1 tabs, got 2 tabs.\n\n".to_string();
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
fn parsing_machine_fail_invalid_network_declaration() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_network_declaration.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_network_declaration.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 11: Unable to parse inside of Networks due to: \n\t\t\tLine 12: expected type Network and got type IP instead.\n\n".to_string();
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
fn parsing_machine_fail_invalid_machine_declaration() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_machine_declaration.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_machine_declaration.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 18: expected type Machine and got type Network instead.\n\n".to_string();
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
fn parsing_machine_fail_invalid_networks_type() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_networks_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_networks_type.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 11: Unexpected type Network.\n\n".to_string();
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
fn parsing_machine_fail_invalid_protocols_type() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_protocols_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_protocols_type.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 13: Unexpected type Protocol.\n\n".to_string();
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
fn parsing_machine_fail_invalid_protocol_type() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_protocol_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_protocol_type.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 13: Unable to parse inside of Protocols due to: \n\t\t\tLine 14: expected type Protocol and got type Protocols instead.\n\n".to_string();
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
fn parsing_machine_fail_invalid_applications_type() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_applications_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_applications_type.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 16: Unexpected type Application.\n\n".to_string();
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
fn parsing_machine_fail_invalid_application_type() {
    let result = parser_testing("./tests/parsing_tests/machine_fail_invalid_application_type.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_fail_invalid_application_type.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 16: Unable to parse inside of Applications due to: \n\t\t\tLine 16: expected type Application and got type Network instead.\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}