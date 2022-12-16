use std::fs;
use elvis::parsing::core_parser;
use elvis::parsing::parsing_data::*;

/// main wrapper for parsing testing.
fn parser_testing(file_path: &str) -> Result<String, String> {
    let contents = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
    let fixed_string = contents.replace('\r', "");
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
#[test]
fn parsing_test_correct() {
    let result = parser_testing("./tests/parsing_tests/test1.txt");
    let s = Sim { 
        networks: vec![
            Network { 
                dectype: DecType::Network, 
                options: vec![("id", "5")], 
                ip: vec![
                    IP { 
                        dectype: DecType::IP, 
                        options: vec![("range", "123.45.67.89-123.45.67.91")] 
                    }, 
                    IP { 
                        dectype: DecType::IP, 
                        options: vec![("range", "123.45.67.92-123.45.67.94")] 
                    }, 
                    IP { 
                        dectype: DecType::IP, 
                        options: vec![("ip", "192.168.1.121")] 
                    }
                ] 
            }, 
            Network { 
                dectype: DecType::Network, 
                options: vec![("id", "1")], 
                ip: vec![
                    IP { 
                        dectype: DecType::IP, 
                        options: vec![("range", "12.34.56.789-14.34.56.789")] 
                    }
                ] 
            }
        ], 
        machines: vec![
            Machine { 
                dectype: DecType::Machine, 
                options: Some(vec![
                    ("name", "test")
                ]), 
                interfaces: Interfaces { 
                    networks: vec![
                        MachineNetwork { 
                            dectype: DecType::Network, 
                            options: vec![
                                ("id", "5")
                            ] 
                        }
                    ], 
                    protocols: vec![
                        Protocol { 
                            dectype: DecType::Protocol, 
                            options: vec![
                                ("name", "IPv4")
                            ] 
                        }, Protocol { 
                            dectype: DecType::Protocol, 
                            options: vec![
                                ("name", "TCP")
                            ] 
                        }
                    ], 
                    applications: vec![
                        Application { 
                            dectype: DecType::Application, 
                            options: vec![
                                ("name", "send_message"), 
                                ("message", "Hello!"), 
                                ("to", "10.0.0.1")
                                ] 
                        }
                    ] 
                } 
            }
        ]
    };
    
    assert_eq!(result.unwrap(), format!("{:?}", s))
}
