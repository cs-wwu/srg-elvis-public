//! Tests for valid parsing
#![cfg(test)]

use crate::ndl::core_parser;
use crate::ndl::parsing::parsing_data::*;
use std::collections::HashMap;

#[test]
fn basic() {
    let result = core_parser(include_str!("basic.ndl"));
    let s = Sim {
        networks: HashMap::from([
            (
                "5".to_string(),
                Network {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "5".to_string())]),
                    ip: vec![
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "range".to_string(),
                                "123.45.67.89/91".to_string(),
                            )]),
                        },
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "range".to_string(),
                                "123.45.67.92/94".to_string(),
                            )]),
                        },
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "ip".to_string(),
                                "192.168.1.121".to_string(),
                            )]),
                        },
                    ],
                },
            ),
            (
                "1".to_string(),
                Network {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "1".to_string())]),
                    ip: vec![IP {
                        dectype: DecType::IP,
                        options: HashMap::from([(
                            "range".to_string(),
                            "12.34.56.78/89".to_string(),
                        )]),
                    }],
                },
            ),
        ]),
        machines: vec![Machine {
            dectype: DecType::Machine,
            options: Some(HashMap::from([("name".to_string(), "test".to_string())])),
            interfaces: Interfaces {
                networks: vec![MachineNetwork {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "5".to_string())]),
                }],
                protocols: vec![
                    Protocol {
                        dectype: DecType::Protocol,
                        options: HashMap::from([("name".to_string(), "IPv4".to_string())]),
                    },
                    Protocol {
                        dectype: DecType::Protocol,
                        options: HashMap::from([("name".to_string(), "TCP".to_string())]),
                    },
                ],
                applications: vec![Application {
                    dectype: DecType::Application,
                    options: HashMap::from([
                        ("name".to_string(), "send_message".to_string()),
                        ("message".to_string(), "Hello!".to_string()),
                        ("to".to_string(), "10.0.0.1".to_string()),
                    ]),
                }],
            },
        }],
    };

    assert_eq!(result.unwrap(), s)
}

#[test]
fn new_line() {
    let result = core_parser(include_str!("new_line.ndl"));
    let s = Sim {
        networks: HashMap::from([
            (
                "5".to_string(),
                Network {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "5".to_string())]),
                    ip: vec![
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "range".to_string(),
                                "123.45.67.89/91".to_string(),
                            )]),
                        },
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "range".to_string(),
                                "123.45.67.92/94".to_string(),
                            )]),
                        },
                        IP {
                            dectype: DecType::IP,
                            options: HashMap::from([(
                                "ip".to_string(),
                                "192.168.1.121".to_string(),
                            )]),
                        },
                    ],
                },
            ),
            (
                "1".to_string(),
                Network {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "1".to_string())]),
                    ip: vec![IP {
                        dectype: DecType::IP,
                        options: HashMap::from([(
                            "range".to_string(),
                            "12.34.56.78/89".to_string(),
                        )]),
                    }],
                },
            ),
        ]),
        machines: vec![Machine {
            dectype: DecType::Machine,
            options: Some(HashMap::from([("name".to_string(), "test".to_string())])),
            interfaces: Interfaces {
                networks: vec![MachineNetwork {
                    dectype: DecType::Network,
                    options: HashMap::from([("id".to_string(), "5".to_string())]),
                }],
                protocols: vec![
                    Protocol {
                        dectype: DecType::Protocol,
                        options: HashMap::from([("name".to_string(), "IPv4".to_string())]),
                    },
                    Protocol {
                        dectype: DecType::Protocol,
                        options: HashMap::from([("name".to_string(), "TCP".to_string())]),
                    },
                ],
                applications: vec![Application {
                    dectype: DecType::Application,
                    options: HashMap::from([
                        ("name".to_string(), "send_message".to_string()),
                        ("message".to_string(), "Hello!".to_string()),
                        ("to".to_string(), "10.0.0.1".to_string()),
                    ]),
                }],
            },
        }],
    };

    assert_eq!(result.unwrap(), s)
}
