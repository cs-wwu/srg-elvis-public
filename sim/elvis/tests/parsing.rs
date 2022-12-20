// use std::collections::HashMap;
use std::fs;
use elvis::parsing::core_parser;
// use elvis::parsing::parsing_data::*;

/// main wrapper for parsing testing.
fn parser_testing(file_path: &str) -> Result<String, String> {
    let contents = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
    let fixed_string = contents.replace('\r', "").replace("    ", "\t");
    let res = core_parser(&fixed_string, file_path);
    match res {
        Ok(s) => {
            return Ok(format!("{:?}", s));
        }

        Err(e) => {
            return Err(e);
        }
    }
}
// TODO: Test broken as hashmap ordering comparisons as strings
// TODO: Need to fix borrowed property so we can compare sim's directly
// #[test]
// fn parsing_test_correct() {
//     let result = parser_testing("./tests/parsing_tests/test1.txt");
//     let s = Sim { 
//         networks: HashMap::from([
//             ("5", Network { 
//                 dectype: DecType::Network, 
//                 options: HashMap::from([("id", "5")]), 
//                 ip: vec![
//                     IP { 
//                         dectype: DecType::IP, 
//                         options: HashMap::from([("range", "123.45.67.89-123.45.67.91")])
//                     }, 
//                     IP { 
//                         dectype: DecType::IP, 
//                         options: HashMap::from([("range", "123.45.67.92-123.45.67.94")])
//                     }, 
//                     IP { 
//                         dectype: DecType::IP, 
//                         options: HashMap::from([("ip", "192.168.1.121")])
//                     }
//                 ] 
//             }), 
//             ("1", Network { 
//                 dectype: DecType::Network, 
//                 options: HashMap::from([("id", "1")]), 
//                 ip: vec![
//                     IP { 
//                         dectype: DecType::IP, 
//                         options: HashMap::from([("range", "12.34.56.789-14.34.56.789")])
//                     }
//                 ] 
//             })
//         ]), 
//         machines: vec![
//             Machine { 
//                 dectype: DecType::Machine, 
//                 options: Some(HashMap::from([
//                     ("name", "test")
//                 ])), 
//                 interfaces: Interfaces { 
//                     networks: vec![
//                         MachineNetwork { 
//                             dectype: DecType::Network, 
//                             options: HashMap::from([
//                                 ("id", "5")
//                             ])
//                         }
//                     ], 
//                     protocols: vec![
//                         Protocol { 
//                             dectype: DecType::Protocol, 
//                             options: HashMap::from([
//                                 ("name", "IPv4")
//                             ])
//                         }, Protocol { 
//                             dectype: DecType::Protocol, 
//                             options: HashMap::from([
//                                 ("name", "TCP")
//                             ])
//                         }
//                     ], 
//                     applications: vec![
//                         Application { 
//                             dectype: DecType::Application, 
//                             options: HashMap::from([
//                                 ("name", "send_message"), 
//                                 ("message", "Hello!"), 
//                                 ("to", "10.0.0.1")
//                                 ])
//                         }
//                     ] 
//                 } 
//             }
//         ]
//     };
    
//     assert_eq!(result.unwrap(), format!("{:?}", s))
// }

#[test]
fn parsing_test_network_fail_1() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_1.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_test_fail_1.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \n\t\tLine 3: expected type IP and got type Network instead.\n\n".to_string();
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
fn parsing_test_network_fail_2() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_2.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_test_fail_2.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 3: Unable to parse inside of Network due to: \n\t\tLine 3: expected 2 tabs and got 3 tabs instead.\n\n".to_string();
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
fn parsing_test_network_fail_3() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_3.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_test_fail_3.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 4: expected type Network and got type Networks instead.\n\n".to_string();
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
fn parsing_test_network_fail_4() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_4.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_test_fail_4.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 2: Unable to parse inside of Network due to: \nLine 5: extra argument at 'S ip='192.168.1.121''\n\n".to_string();
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
fn parsing_test_network_fail_5() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_5.txt");
    let s: String = "Errors at ./tests/parsing_tests/network_test_fail_5.txt:\n\nLine 1: Unable to insert Network into Networks due to duplicate id: 5\n".to_string();
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
fn parsing_test_network_fail_6() {
    let result = parser_testing("./tests/parsing_tests/network_test_fail_6.txt");
    let s: String = "Line 21: Unable to insert Network into Networks due to duplicate id: 5".to_string();
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
fn parsing_test_machine_fail_1() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_1.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_1.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tFailed to include all required types for machine. Still needs types: [Applications]\n".to_string();
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
fn parsing_test_machine_fail_2() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_2.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_2.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Invalid tab count. Expected 1 tabs, got 2 tabs.\n\n".to_string();
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
fn parsing_test_machine_fail_3() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_3.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_3.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 11: Unable to parse inside of Networks due to: \n\t\t\tLine 12: expected type Network and got type IP instead.\n\n".to_string();
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
fn parsing_test_machine_fail_4() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_4.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_4.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 18: expected type Machine and got type Network instead.\n\n".to_string();
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
fn parsing_test_machine_fail_5() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_5.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_5.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 11: Unexpected type Network.\n\n".to_string();
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
fn parsing_test_machine_fail_6() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_6.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_6.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 13: Unexpected type Protocol.\n\n".to_string();
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
fn parsing_test_machine_fail_7() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_7.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_7.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 13: Unable to parse inside of Protocols due to: \n\t\t\tLine 14: expected type Protocol and got type Protocols instead.\n\n".to_string();
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
fn parsing_test_machine_fail_8() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_8.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_8.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\tLine 16: Unexpected type Application.\n\n".to_string();
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
fn parsing_test_machine_fail_9() {
    let result = parser_testing("./tests/parsing_tests/machine_test_fail_9.txt");
    let s: String = "Errors at ./tests/parsing_tests/machine_test_fail_9.txt:\n\nLine 9: Unable to parse inside of Machines due to: \n\tLine 10: Unable to parse inside of Machine due to: \n\t\t\t\tLine 16: Unable to parse inside of Applications due to: \n\t\t\tLine 16: expected type Application and got type Network instead.\n\n".to_string();
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
fn parsing_test_options_fail_1() {
    let result = parser_testing("./tests/parsing_tests/options_test_fail_1.txt");
    let s: String = "Errors at ./tests/parsing_tests/options_test_fail_1.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 6: duplicate argument 'id'='5'\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}